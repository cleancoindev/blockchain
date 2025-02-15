use std::ffi::CString;
use std::fmt;
use std::str;

use hex::FromHexError;
use assets::AssetIdError;
use libc::c_char;

#[derive(Debug)]
pub struct Error {
    message: Option<CString>,
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    None,
    Utf8(str::Utf8Error),
    Hex(FromHexError),
    Text(String),
    Asset(AssetIdError),
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error {
        Error {
            message: None,
            kind: kind,
        }
    }

    pub fn is_err(&self) -> bool {
        match self.kind {
            ErrorKind::None => false,
            ErrorKind::Utf8(_) | ErrorKind::Hex(_) | ErrorKind::Text(_) | ErrorKind::Asset(_) => {
                true
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::None => write!(f, "no error"),
            ErrorKind::Utf8(ref e) => e.fmt(f),
            ErrorKind::Hex(ref e) => e.fmt(f),
            ErrorKind::Text(ref e) => write!(f, "Error: {}", e),
            ErrorKind::Asset(ref e) => e.fmt(f),
        }
    }
}

ffi_fn! {
    fn dmbc_error_new() -> *mut Error {
        Box::into_raw(Box::new(Error::new(ErrorKind::None)))
    }
}

ffi_fn! {
    fn dmbc_error_free(err: *mut Error) {
        unsafe { Box::from_raw(err); }
    }
}

ffi_fn! {
    fn dmbc_error_message(err: *mut Error) -> *const c_char {
        let err = unsafe { &mut *err };
        let cmsg = match CString::new(format!("{}", err)) {
            Ok(msg) => msg,
            Err(err) => {
                let nul = err.nul_position();
                let msg = err.into_vec();
                CString::new(msg[0..nul].to_owned()).unwrap()
            }
        };
        let p = cmsg.as_ptr();
        err.message = Some(cmsg);
        p
    }
}
