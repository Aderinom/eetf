use super::*;
use crate::convert::TryAsRef;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use libflate::zlib;
use num::bigint::BigInt;
use std::convert::From;
use std::io;
use std::io::Write;
use std::str;

/// Errors which can occur when decoding a term
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("I/O error")]
    Io(#[from] io::Error),

    #[error("the format version {version} is unsupported")]
    UnsupportedVersion { version: u8 },

    #[error("unknown tag {tag}")]
    UnknownTag { tag: u8 },

    #[error("{value} is not a {expected}")]
    UnexpectedType { value: Term, expected: String },

    #[error("{value} is out of range {range:?}")]
    OutOfRange {
        value: i32,
        range: std::ops::Range<i32>,
    },

    #[error("tried to convert non-finite float")]
    NonFiniteFloat,
}

/// Errors which can occur when encoding a term
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("I/O error")]
    Io(#[from] io::Error),

    #[error("too long atom name: {} bytes", .0.name.len())]
    TooLongAtomName(Atom),

    #[error("too large integer value: {} bytes required to encode", .0.value.to_bytes_le().1.len())]
    TooLargeInteger(BigInteger),

    #[error("too large reference ID: {} bytes required to encode", .0.id.len() * 4)]
    TooLargeReferenceId(Reference),
}

pub type DecodeResult = Result<Term, DecodeError>;
pub type EncodeResult = Result<(), EncodeError>;

pub(crate) const VERSION: u8 = 131;

pub(crate) const DISTRIBUTION_HEADER: u8 = 68;
pub(crate) const NEW_FLOAT_EXT: u8 = 70;
pub(crate) const BIT_BINARY_EXT: u8 = 77;
pub(crate) const COMPRESSED_TERM: u8 = 80;
pub(crate) const ATOM_CACHE_REF: u8 = 82;
pub(crate) const NEW_PID_EXT: u8 = 88;
pub(crate) const NEW_PORT_EXT: u8 = 89;
pub(crate) const NEWER_REFERENCE_EXT: u8 = 90;
pub(crate) const SMALL_INTEGER_EXT: u8 = 97;
pub(crate) const INTEGER_EXT: u8 = 98;
pub(crate) const FLOAT_EXT: u8 = 99;
pub(crate) const ATOM_EXT: u8 = 100; // deprecated
pub(crate) const REFERENCE_EXT: u8 = 101; // deprecated
pub(crate) const PORT_EXT: u8 = 102;
pub(crate) const PID_EXT: u8 = 103;
pub(crate) const SMALL_TUPLE_EXT: u8 = 104;
pub(crate) const LARGE_TUPLE_EXT: u8 = 105;
pub(crate) const NIL_EXT: u8 = 106;
pub(crate) const STRING_EXT: u8 = 107;
pub(crate) const LIST_EXT: u8 = 108;
pub(crate) const BINARY_EXT: u8 = 109;
pub(crate) const SMALL_BIG_EXT: u8 = 110;
pub(crate) const LARGE_BIG_EXT: u8 = 111;
pub(crate) const NEW_FUN_EXT: u8 = 112;
pub(crate) const EXPORT_EXT: u8 = 113;
pub(crate) const NEW_REFERENCE_EXT: u8 = 114;
pub(crate) const SMALL_ATOM_EXT: u8 = 115; // deprecated
pub(crate) const MAP_EXT: u8 = 116;
pub(crate) const FUN_EXT: u8 = 117;
pub(crate) const ATOM_UTF8_EXT: u8 = 118;
pub(crate) const SMALL_ATOM_UTF8_EXT: u8 = 119;
pub(crate) const V4_PORT_EXT: u8 = 120;

pub(crate) mod aux {
    use num::bigint::Sign;
    use std::io;
    use std::ops::Range;
    use std::str;

    pub fn term_into_atom(t: crate::Term) -> Result<crate::Atom, super::DecodeError> {
        t.try_into()
            .map_err(|t| super::DecodeError::UnexpectedType {
                value: t,
                expected: "Atom".to_string(),
            })
    }
    pub fn term_into_pid(t: crate::Term) -> Result<crate::Pid, super::DecodeError> {
        t.try_into()
            .map_err(|t| super::DecodeError::UnexpectedType {
                value: t,
                expected: "Pid".to_string(),
            })
    }
    pub fn term_into_fix_integer(t: crate::Term) -> Result<crate::FixInteger, super::DecodeError> {
        t.try_into()
            .map_err(|t| super::DecodeError::UnexpectedType {
                value: t,
                expected: "FixInteger".to_string(),
            })
    }
    pub fn term_into_ranged_integer(
        t: crate::Term,
        range: Range<i32>,
    ) -> Result<i32, super::DecodeError> {
        term_into_fix_integer(t).and_then(|i| {
            let n = i.value;
            if range.start <= n && n <= range.end {
                Ok(n)
            } else {
                Err(super::DecodeError::OutOfRange { value: n, range })
            }
        })
    }
    pub fn invalid_data_error<T>(message: String) -> io::Result<T> {
        Err(io::Error::new(io::ErrorKind::InvalidData, message))
    }
    pub fn other_error<T>(message: String) -> io::Result<T> {
        Err(io::Error::new(io::ErrorKind::Other, message))
    }
    pub fn latin1_bytes_to_string(buf: &[u8]) -> io::Result<String> {
        // FIXME: Supports Latin1 characters
        str::from_utf8(buf)
            .or_else(|e| other_error(e.to_string()))
            .map(ToString::to_string)
    }
    pub fn byte_to_sign(b: u8) -> io::Result<Sign> {
        match b {
            0 => Ok(Sign::Plus),
            1 => Ok(Sign::Minus),
            _ => invalid_data_error(format!("A sign value must be 0 or 1: value={}", b)),
        }
    }
    pub fn sign_to_byte(sign: Sign) -> u8 {
        if sign == Sign::Minus {
            1
        } else {
            0
        }
    }
}
