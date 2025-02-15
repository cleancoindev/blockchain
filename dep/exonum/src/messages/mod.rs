// Copyright 2017 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Consensus and other messages and related utilities.

use std::fmt;

use bit_vec::BitVec;

use crypto::PublicKey;
use encoding::Error;
use helpers::{Height, Round, ValidatorId};

pub use self::protocol::*;
pub use self::raw::{
    Message, MessageBuffer, MessageWriter, RawMessage, HEADER_LENGTH, PROTOCOL_MAJOR_VERSION,
    TEST_NETWORK_ID,
};

#[macro_use]
mod spec;
mod protocol;
mod raw;

#[cfg(test)]
mod tests;

// TODO: implement common methods for enum types (hash, raw, from_raw, verify)
// TODO: use macro for implementing enums (ECR-166)

/// Raw transaction type.
pub type RawTransaction = RawMessage;

/// Any possible message.
#[derive(Debug, Clone, PartialEq)]
pub enum Any {
    /// `Connect` message.
    Connect(Connect),
    /// `Status` message.
    Status(Status),
    /// `Block` message.
    Block(BlockResponse),
    /// Consensus message.
    Consensus(ConsensusMessage),
    /// Request for the some data.
    Request(RequestMessage),
    /// Transaction.
    Transaction(RawTransaction),
}

/// Consensus message.
#[derive(Clone, PartialEq)]
pub enum ConsensusMessage {
    /// `Propose` message.
    Propose(Propose),
    /// `Prevote` message.
    Prevote(Prevote),
    /// `Precommit` message.
    Precommit(Precommit),
}

/// A request for the some data.
#[derive(Clone, PartialEq)]
pub enum RequestMessage {
    /// Propose request.
    Propose(ProposeRequest),
    /// Transactions request.
    Transactions(TransactionsRequest),
    /// Prevotes request.
    Prevotes(PrevotesRequest),
    /// Peers request.
    Peers(PeersRequest),
    /// Block request.
    Block(BlockRequest),
}

impl RequestMessage {
    /// Returns public key of the message sender.
    pub fn from(&self) -> &PublicKey {
        match *self {
            RequestMessage::Propose(ref msg) => msg.from(),
            RequestMessage::Transactions(ref msg) => msg.from(),
            RequestMessage::Prevotes(ref msg) => msg.from(),
            RequestMessage::Peers(ref msg) => msg.from(),
            RequestMessage::Block(ref msg) => msg.from(),
        }
    }

    /// Returns public key of the message recipient.
    pub fn to(&self) -> &PublicKey {
        match *self {
            RequestMessage::Propose(ref msg) => msg.to(),
            RequestMessage::Transactions(ref msg) => msg.to(),
            RequestMessage::Prevotes(ref msg) => msg.to(),
            RequestMessage::Peers(ref msg) => msg.to(),
            RequestMessage::Block(ref msg) => msg.to(),
        }
    }

    /// Verifies the message signature with given public key.
    #[cfg_attr(feature = "flame_profile", flame)]
    pub fn verify(&self, public_key: &PublicKey) -> bool {
        match *self {
            RequestMessage::Propose(ref msg) => msg.verify_signature(public_key),
            RequestMessage::Transactions(ref msg) => msg.verify_signature(public_key),
            RequestMessage::Prevotes(ref msg) => msg.verify_signature(public_key),
            RequestMessage::Peers(ref msg) => msg.verify_signature(public_key),
            RequestMessage::Block(ref msg) => msg.verify_signature(public_key),
        }
    }

    /// Returns raw message.
    pub fn raw(&self) -> &RawMessage {
        match *self {
            RequestMessage::Propose(ref msg) => msg.raw(),
            RequestMessage::Transactions(ref msg) => msg.raw(),
            RequestMessage::Prevotes(ref msg) => msg.raw(),
            RequestMessage::Peers(ref msg) => msg.raw(),
            RequestMessage::Block(ref msg) => msg.raw(),
        }
    }
}

impl fmt::Debug for RequestMessage {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            RequestMessage::Propose(ref msg) => write!(fmt, "{:?}", msg),
            RequestMessage::Transactions(ref msg) => write!(fmt, "{:?}", msg),
            RequestMessage::Prevotes(ref msg) => write!(fmt, "{:?}", msg),
            RequestMessage::Peers(ref msg) => write!(fmt, "{:?}", msg),
            RequestMessage::Block(ref msg) => write!(fmt, "{:?}", msg),
        }
    }
}

impl ConsensusMessage {
    /// Returns validator id of the message sender.
    pub fn validator(&self) -> ValidatorId {
        match *self {
            ConsensusMessage::Propose(ref msg) => msg.validator(),
            ConsensusMessage::Prevote(ref msg) => msg.validator(),
            ConsensusMessage::Precommit(ref msg) => msg.validator(),
        }
    }

    /// Returns height of the message.
    pub fn height(&self) -> Height {
        match *self {
            ConsensusMessage::Propose(ref msg) => msg.height(),
            ConsensusMessage::Prevote(ref msg) => msg.height(),
            ConsensusMessage::Precommit(ref msg) => msg.height(),
        }
    }

    /// Returns round of the message.
    pub fn round(&self) -> Round {
        match *self {
            ConsensusMessage::Propose(ref msg) => msg.round(),
            ConsensusMessage::Prevote(ref msg) => msg.round(),
            ConsensusMessage::Precommit(ref msg) => msg.round(),
        }
    }

    /// Returns raw message.
    pub fn raw(&self) -> &RawMessage {
        match *self {
            ConsensusMessage::Propose(ref msg) => msg.raw(),
            ConsensusMessage::Prevote(ref msg) => msg.raw(),
            ConsensusMessage::Precommit(ref msg) => msg.raw(),
        }
    }

    /// Verifies the message signature with given public key.
    pub fn verify(&self, public_key: &PublicKey) -> bool {
        match *self {
            ConsensusMessage::Propose(ref msg) => msg.verify_signature(public_key),
            ConsensusMessage::Prevote(ref msg) => msg.verify_signature(public_key),
            ConsensusMessage::Precommit(ref msg) => msg.verify_signature(public_key),
        }
    }
}

impl fmt::Debug for ConsensusMessage {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ConsensusMessage::Propose(ref msg) => write!(fmt, "{:?}", msg),
            ConsensusMessage::Prevote(ref msg) => write!(fmt, "{:?}", msg),
            ConsensusMessage::Precommit(ref msg) => write!(fmt, "{:?}", msg),
        }
    }
}

impl Any {
    /// Converts the `RawMessage` to the `Any` message.
    pub fn from_raw(raw: RawMessage) -> Result<Any, Error> {
        // TODO: check input message size (ECR-166)
        let msg = if raw.service_id() == CONSENSUS {
            match raw.message_type() {
                CONNECT_MESSAGE_ID => Any::Connect(Connect::from_raw(raw)?),
                STATUS_MESSAGE_ID => Any::Status(Status::from_raw(raw)?),
                BLOCK_RESPONSE_MESSAGE_ID => Any::Block(BlockResponse::from_raw(raw)?),

                PROPOSE_MESSAGE_ID => {
                    Any::Consensus(ConsensusMessage::Propose(Propose::from_raw(raw)?))
                }
                PREVOTE_MESSAGE_ID => {
                    Any::Consensus(ConsensusMessage::Prevote(Prevote::from_raw(raw)?))
                }
                PRECOMMIT_MESSAGE_ID => {
                    Any::Consensus(ConsensusMessage::Precommit(Precommit::from_raw(raw)?))
                }

                PROPOSE_REQUEST_MESSAGE_ID => {
                    Any::Request(RequestMessage::Propose(ProposeRequest::from_raw(raw)?))
                }
                TRANSACTIONS_REQUEST_MESSAGE_ID => Any::Request(RequestMessage::Transactions(
                    TransactionsRequest::from_raw(raw)?,
                )),
                PREVOTES_REQUEST_MESSAGE_ID => {
                    Any::Request(RequestMessage::Prevotes(PrevotesRequest::from_raw(raw)?))
                }
                PEERS_REQUEST_MESSAGE_ID => {
                    Any::Request(RequestMessage::Peers(PeersRequest::from_raw(raw)?))
                }
                BLOCK_REQUEST_MESSAGE_ID => {
                    Any::Request(RequestMessage::Block(BlockRequest::from_raw(raw)?))
                }

                message_type => {
                    return Err(Error::IncorrectMessageType { message_type });
                }
            }
        } else {
            Any::Transaction(raw)
        };
        Ok(msg)
    }
}
