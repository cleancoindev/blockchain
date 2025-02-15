use std::ffi::CStr;

use assets::AssetId;
use crypto::{PublicKey, Signature};
use hex::FromHex;
use libc::{c_char, size_t};

use error::{Error, ErrorKind};

pub fn hex_string(bytes: Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    strs.join("")
}

pub fn parse_str<'a>(string: *const c_char) -> Result<&'a str, Error> {
    match unsafe { CStr::from_ptr(string).to_str() } {
        Ok(string_str) => Ok(string_str),
        Err(err) => Err(Error::new(ErrorKind::Utf8(err))),
    }
}

pub fn parse_public_key(public_key: *const c_char) -> Result<PublicKey, Error> {
    let pk_str = parse_str(public_key)?;
    match PublicKey::from_hex(pk_str) {
        Ok(pk) => Ok(pk),
        Err(err) => Err(Error::new(ErrorKind::Hex(err))),
    }
}

pub fn parse_signature(signature: *const c_char) -> Result<Signature, Error> {
    let sig_str = parse_str(signature)?;
    match Signature::from_hex(sig_str) {
        Ok(sig) => Ok(sig),
        Err(err) => Err(Error::new(ErrorKind::Hex(err))),
    }
}

pub fn parse_asset_id(asset_id: *const c_char) -> Result<AssetId, Error> {
    let asset_id_str = parse_str(asset_id)?;
    match AssetId::from_hex(asset_id_str) {
        Ok(asset_id) => Ok(asset_id),
        Err(err) => Err(Error::new(ErrorKind::Asset(err))),
    }
}

ffi_fn! {
    fn dmbc_bytes_free(ptr: *mut u8, len: size_t) {
        let len = len as usize;
        unsafe {
            drop(Vec::from_raw_parts(ptr, len, len));
        }
    }
}
