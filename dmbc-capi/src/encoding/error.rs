use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;

use super::Offset;

#[derive(Debug)]
/// This structure represent `encoding` specific errors.
/// This errors returned by function `check` of each `Field`.
pub enum Error {
    /// Payload is short for this message.
    UnexpectedlyShortPayload {
        /// real message size
        actual_size: Offset,
        /// expected size of fixed part
        minimum_size: Offset,
    },
    /// Boolean value is incorrect
    IncorrectBoolean {
        /// position in buffer where error appears.
        position: Offset,
        /// value that was parsed as bool
        value: u8,
    },
    /// Unsupported floating point value (Infinity, NaN or signaling NaN).
    UnsupportedFloat {
        /// Position in buffer where error appears.
        position: Offset,
        /// Value represented as `f64`.
        value: f64,
    },
    /// Segment reference is incorrect
    IncorrectSegmentReference {
        /// position in buffer where error appears.
        position: Offset,
        /// value that was parsed as segment reference
        value: Offset,
    },
    /// Segment size is incorrect
    IncorrectSegmentSize {
        /// position in buffer where error appears.
        position: Offset,
        /// value that was parsed as size
        value: Offset,
    },
    /// `RawMessage` is to short
    UnexpectedlyShortRawMessage {
        /// position in buffer where error appears.
        position: Offset,
        /// size of raw message in buffer
        size: Offset,
    },
    /// Incorrect size of `RawMessage` found in buffer
    IncorrectSizeOfRawMessage {
        /// position in buffer where error appears.
        position: Offset,
        /// parsed message size
        actual_size: Offset,
        /// expected fixed part message size
        declared_size: Offset,
    },
    /// Incorrect `message_id` found in buffer.
    IncorrectMessageType {
        /// expected `message_id`
        message_type: u16,
    },
    /// Incorrect `service_id` found in buffer.
    IncorrectServiceId {
        /// expected `service_id`
        service_id: u16,
    },
    /// Incorrect `network_id` found in buffer.
    IncorrectNetworkId {
        /// expected `network_id`
        network_id: u8,
    },
    /// Unsupported message version.
    UnsupportedProtocolVersion {
        /// Actual message version.
        version: u8,
    },
    /// Different segments overlaps
    OverlappingSegment {
        /// last segment ended position
        last_end: Offset,
        /// start of new segment
        start: Offset,
    },
    /// Spaces found between segments
    SpaceBetweenSegments {
        /// last segment ended position
        last_end: Offset,
        /// start of new segment
        start: Offset,
    },
    /// Error in parsing `Utf8` `String`
    Utf8 {
        /// position in buffer where error appears.
        position: Offset,
        /// what error exact was
        error: ::std::str::Utf8Error,
    },
    /// Overflow in Offsets
    OffsetOverflow,
    /// Basic error support, for custom fields.
    Basic(Cow<'static, str>),
    /// Other error for custom fields
    Other(Box<StdError>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = {:?}", self.description(), self)
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::UnexpectedlyShortPayload { .. } => "Unexpectedly short payload",
            Error::IncorrectBoolean { .. } => "Incorrect boolean value",
            Error::UnsupportedFloat { .. } => "Unsupported float value",
            Error::IncorrectSegmentReference { .. } => "Incorrect segment reference",
            Error::IncorrectSegmentSize { .. } => "Incorrect segment size",
            Error::UnexpectedlyShortRawMessage { .. } => "Unexpectedly short RawMessage",
            Error::IncorrectSizeOfRawMessage { .. } => "Incorrect size of RawMessage",
            Error::IncorrectMessageType { .. } => "Incorrect message type",
            Error::IncorrectServiceId { .. } => "Incorrect service id",
            Error::IncorrectNetworkId { .. } => "Incorrect network id",
            Error::UnsupportedProtocolVersion { .. } => "Unsupported protocol version",
            Error::OverlappingSegment { .. } => "Overlapping segments",
            Error::SpaceBetweenSegments { .. } => "Space between segments",
            Error::Utf8 { .. } => "Utf8 error in parsing string",
            Error::OffsetOverflow => "Offset pointers overflow",
            Error::Basic(ref x) => x.as_ref(),
            Error::Other(_) => "Other error",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        use std::ops::Deref;
        if let Error::Other(ref error) = *self {
            Some(error.deref())
        } else {
            None
        }
    }
}

impl From<Box<StdError>> for Error {
    fn from(t: Box<StdError>) -> Error {
        Error::Other(t)
    }
}

impl From<Cow<'static, str>> for Error {
    fn from(t: Cow<'static, str>) -> Error {
        Error::Basic(t)
    }
}

impl From<&'static str> for Error {
    fn from(t: &'static str) -> Error {
        Error::Basic(t.into())
    }
}
