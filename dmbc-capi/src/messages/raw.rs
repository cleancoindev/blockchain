use std::fmt::Debug;
use std::ops::Deref;
use std::{convert, mem, sync};

use byteorder::{ByteOrder, LittleEndian};

use crypto::{Signature, SIGNATURE_LENGTH};
use encoding::{self, CheckedOffset, Field, Offset, Result as StreamStructResult};

/// Length of the message header.
pub const HEADER_LENGTH: usize = 10;
// TODO: Better name (ECR-166).
#[doc(hidden)]
pub const TEST_NETWORK_ID: u8 = 0;
/// Version of the protocol. Different versions are incompatible.
pub const PROTOCOL_MAJOR_VERSION: u8 = 0;

/// Thread-safe reference-counting pointer to the `MessageBuffer`.
#[derive(Debug, Clone, PartialEq)]
pub struct RawMessage(sync::Arc<MessageBuffer>);

impl RawMessage {
    /// Creates a new `RawMessage` instance with the given `MessageBuffer`.
    pub fn new(buffer: MessageBuffer) -> Self {
        RawMessage(sync::Arc::new(buffer))
    }

    /// Creates a new `RawMessage` instance from the given `Vec<u8>`.
    pub fn from_vec(vec: Vec<u8>) -> Self {
        RawMessage(sync::Arc::new(MessageBuffer::from_vec(vec)))
    }
}

impl Deref for RawMessage {
    type Target = MessageBuffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for RawMessage {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

// TODO: reduce `to` argument from `write`, `read` and `check` methods
// TODO: payload_length as a first value into message header
// TODO: make sure that message length is enough when using mem::transmute
// (ECR-166)

/// A raw message represented by the bytes buffer.
#[derive(Debug, PartialEq)]
pub struct MessageBuffer {
    raw: Vec<u8>,
}

impl MessageBuffer {
    /// Creates `MessageBuffer` instance from the bytes vector.
    ///
    /// # Example
    ///
    /// ```
    /// use exonum::messages::MessageBuffer;
    ///
    /// let message_buffer = MessageBuffer::from_vec(vec![1, 2, 3]);
    /// assert!(!message_buffer.is_empty());
    /// ```
    pub fn from_vec(raw: Vec<u8>) -> MessageBuffer {
        // TODO: check that size >= HEADER_LENGTH
        // TODO: check that payload_length == raw.len()
        // ECR-166
        MessageBuffer { raw: raw }
    }

    /// Returns the length of the message in bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use exonum::messages::MessageBuffer;
    ///
    /// let data = vec![1, 2, 3];
    /// let message_buffer = MessageBuffer::from_vec(data.clone());
    /// assert_eq!(data.len(), message_buffer.len());
    /// ```
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Returns `true` if the `MessageBuffer` contains no bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use exonum::messages::MessageBuffer;
    ///;
    /// let message_buffer = MessageBuffer::from_vec(vec![]);
    /// assert!(message_buffer.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Returns network id.
    pub fn network_id(&self) -> u8 {
        self.raw[0]
    }

    /// Returns the protocol version.
    pub fn version(&self) -> u8 {
        self.raw[1]
    }

    /// Returns id of the service.
    pub fn service_id(&self) -> u16 {
        LittleEndian::read_u16(&self.raw[4..6])
    }

    /// Returns type of the message.
    pub fn message_type(&self) -> u16 {
        LittleEndian::read_u16(&self.raw[2..4])
    }

    /// Returns message body without signature.
    pub fn body(&self) -> &[u8] {
        &self.raw[..self.raw.len()]
    }

    /// Returns signature of the message.
    pub fn signature(&self) -> &Signature {
        let sign_idx = self.raw.len() - SIGNATURE_LENGTH;
        unsafe { mem::transmute(&self.raw[sign_idx]) }
    }

    /// Checks that `Field` can be safely got with specified `from` and `to` offsets.
    pub fn check<'a, F: Field<'a>>(
        &'a self,
        from: CheckedOffset,
        to: CheckedOffset,
        latest_segment: CheckedOffset,
    ) -> StreamStructResult {
        F::check(
            self.body(),
            (from + HEADER_LENGTH as u32)?,
            (to + HEADER_LENGTH as u32)?,
            latest_segment,
        )
    }

    /// Returns `Field` specified by `from` and `to` offsets. Should not be used directly.
    pub unsafe fn read<'a, F: Field<'a>>(&'a self, from: Offset, to: Offset) -> F {
        F::read(
            self.body(),
            from + HEADER_LENGTH as u32,
            to + HEADER_LENGTH as u32,
        )
    }
}

impl convert::AsRef<[u8]> for MessageBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

/// Message writer.
#[derive(Debug, PartialEq)]
pub struct MessageWriter {
    raw: Vec<u8>,
}

impl MessageWriter {
    /// Creates a `MessageWriter` instance with given parameters.
    pub fn new(
        protocol_version: u8,
        network_id: u8,
        service_id: u16,
        message_type: u16,
        payload_length: usize,
    ) -> Self {
        let mut raw = MessageWriter {
            raw: vec![0; HEADER_LENGTH + payload_length],
        };
        raw.set_network_id(network_id);
        raw.set_version(protocol_version);
        raw.set_service_id(service_id);
        raw.set_message_type(message_type);
        raw
    }

    /// Sets network id.
    fn set_network_id(&mut self, network_id: u8) {
        self.raw[0] = network_id
    }

    /// Sets version.
    fn set_version(&mut self, version: u8) {
        self.raw[1] = version
    }

    /// Sets the service id.
    fn set_service_id(&mut self, message_type: u16) {
        LittleEndian::write_u16(&mut self.raw[4..6], message_type)
    }

    /// Sets the message type.
    fn set_message_type(&mut self, message_type: u16) {
        LittleEndian::write_u16(&mut self.raw[2..4], message_type)
    }

    /// Sets the length of the payload.
    fn set_payload_length(&mut self, length: usize) {
        LittleEndian::write_u32(&mut self.raw[6..10], length as u32)
    }

    /// Writes given field to the given offset.
    #[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
    pub fn write<'a, F: Field<'a>>(&'a mut self, field: F, from: Offset, to: Offset) {
        field.write(
            &mut self.raw,
            from + HEADER_LENGTH as Offset,
            to + HEADER_LENGTH as Offset,
        );
    }

    pub fn to_message_buffer(mut self) -> MessageBuffer {
        let payload_length = self.raw.len() + SIGNATURE_LENGTH;
        self.set_payload_length(payload_length);
        MessageBuffer { raw: self.raw }
    }

    /// Appends the given signature to the message.
    pub fn append_signature(mut self, signature: &Signature) -> MessageBuffer {
        let payload_length = self.raw.len() + SIGNATURE_LENGTH;
        self.set_payload_length(payload_length);
        self.raw.extend_from_slice(signature.as_ref());
        debug_assert_eq!(self.raw.len(), payload_length);
        MessageBuffer { raw: self.raw }
    }
}

pub trait Message: Debug + Send + Sync {
    /// Converts the raw message into the specific one.
    fn from_raw(raw: RawMessage) -> Result<Self, encoding::Error>
    where
        Self: Sized;

    /// Returns raw message.
    fn raw(&self) -> &RawMessage;
}

impl Message for RawMessage {
    fn from_raw(raw: RawMessage) -> Result<Self, encoding::Error> {
        Ok(raw)
    }

    fn raw(&self) -> &RawMessage {
        self
    }
}
