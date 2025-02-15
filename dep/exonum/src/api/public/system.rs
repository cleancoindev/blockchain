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

use iron::prelude::*;
use router::Router;
use serde_json;

use api::{Api, ApiError};
use blockchain::{Blockchain, SharedNodeState};
use crypto::Hash;
use encoding::serialize::FromHex;
use explorer::{BlockchainExplorer, TxInfo};
use helpers::Height;
use hyper::header::ContentType;
use iron::headers::AccessControlAllowOrigin;
use iron::status;
use node::state::TxPool;

#[derive(Serialize)]
struct MemPoolTxInfo {
    content: ::serde_json::Value,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum MemPoolResult {
    Unknown,
    MemPool(MemPoolTxInfo),
    Committed(TxInfo),
}

#[derive(Serialize)]
struct MemPoolInfo {
    size: usize,
}

#[doc(hidden)]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct HealthCheckInfo {
    pub connectivity: bool,
    pub overtake: bool,
    pub net_height: Height,
    pub current_height: Height,
}

/// Public system API.
#[derive(Clone, Debug)]
pub struct SystemApi {
    pool: TxPool,
    blockchain: Blockchain,
    shared_api_state: SharedNodeState,
}

impl SystemApi {
    /// Creates a new `private::SystemApi` instance.
    pub fn new(
        pool: TxPool,
        blockchain: Blockchain,
        shared_api_state: SharedNodeState,
    ) -> SystemApi {
        SystemApi {
            pool,
            blockchain,
            shared_api_state,
        }
    }

    fn get_mempool_info(&self) -> MemPoolInfo {
        MemPoolInfo {
            size: self.pool.read().expect("Expected read lock").len(),
        }
    }

    fn get_transaction(&self, hash_str: &str) -> Result<MemPoolResult, ApiError> {
        let hash = Hash::from_hex(hash_str)?;
        self.pool
            .read()
            .expect("Expected read lock")
            .get(&hash)
            .map_or_else(
                || {
                    let explorer = BlockchainExplorer::new(&self.blockchain);
                    Ok(explorer
                        .tx_info(&hash)?
                        .map_or(MemPoolResult::Unknown, MemPoolResult::Committed))
                },
                |o| {
                    Ok(MemPoolResult::MemPool(MemPoolTxInfo {
                        content: o.serialize_field().map_err(ApiError::Serialize)?,
                    }))
                },
            )
    }
}

impl Api for SystemApi {
    fn wire(&self, router: &mut Router) {
        let self_ = self.clone();
        let mempool_info = move |_: &mut Request| -> IronResult<Response> {
            let info = self_.get_mempool_info();
            self_.ok_response(&::serde_json::to_value(info).unwrap())
        };

        let self_ = self.clone();
        let transaction = move |req: &mut Request| -> IronResult<Response> {
            let params = req.extensions.get::<Router>().unwrap();
            match params.find("hash") {
                Some(hash_str) => {
                    let info = self_.get_transaction(hash_str)?;
                    let result = match info {
                        MemPoolResult::Unknown => Self::not_found_response,
                        _ => Self::ok_response,
                    };
                    result(&self_, &::serde_json::to_value(info).unwrap())
                }
                None => Err(ApiError::IncorrectRequest(
                    "Required parameter of transaction 'hash' is missing".into(),
                ))?,
            }
        };

        let self_ = self.clone();
        let healthcheck = move |_: &mut Request| {
            let net_height = *self_.shared_api_state.net_height.read().unwrap();
            let last_block = self_.blockchain.last_block().clone();
            let (status, code) = if net_height > last_block.height() {
                (true, status::ServiceUnavailable)
            } else {
                (false, status::Ok)
            };

            let info = HealthCheckInfo {
                connectivity: !self_.shared_api_state.peers_info().is_empty(),
                overtake: status,
                net_height,
                current_height: last_block.height(),
            };
            let json = &::serde_json::to_value(info).unwrap();
            let mut resp = Response::with((code, serde_json::to_string_pretty(json).unwrap()));
            resp.headers.set(ContentType::json());
            resp.headers.set(AccessControlAllowOrigin::Any);

            Ok(resp)
        };

        router.get("/v1/mempool", mempool_info, "mempool");
        router.get("/v1/transactions/:hash", transaction, "hash");
        router.get("/v1/healthcheck", healthcheck, "healthcheck_info");
    }
}
