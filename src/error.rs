#[derive(Debug)]
pub enum Error {
    Truncated,
    VarintOverflow,
    VarintUnterminated,
    Utf8(std::str::Utf8Error),
    FromHex,
    Decode(base64::DecodeError),
    InvalidPrefix,
    Json(serde_json::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Truncated => {
                write!(f, "notepack string is truncated")
            }
            Error::VarintOverflow => {
                write!(f, "varint overflowed")
            }
            Error::VarintUnterminated => {
                write!(f, "varint is unterminated")
            }
            Error::Utf8(err) => {
                write!(f, "utf8 error: {err}")
            }
            Error::FromHex => {
                write!(f, "error when converting from hex")
            }
            Error::Decode(err) => {
                write!(f, "base64 decode err: {err}")
            }
            Error::InvalidPrefix => {
                write!(f, "String did not start with notepack_")
            }
            Error::Json(err) => {
                write!(f, "json error: {err}")
            }
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::Utf8(err)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Self {
        Error::Decode(err)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(_err: hex::FromHexError) -> Self {
        Error::FromHex
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl std::error::Error for Error {}
