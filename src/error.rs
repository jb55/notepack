#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    Truncated,
    VarintOverflow,
    VarintUnterminated,
    Utf8(std::str::Utf8Error),
    FromHex,
    Decode(base64::DecodeError),
}

impl core::fmt::Display for PackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PackError::Truncated => {
                write!(f, "notepack string is truncated")
            }
            PackError::VarintOverflow => {
                write!(f, "varint overflowed")
            }
            PackError::VarintUnterminated => {
                write!(f, "varint is unterminated")
            }
            PackError::Utf8(err) => {
                write!(f, "utf8 error: {err}")
            }
            PackError::FromHex => {
                write!(f, "error when converting from hex")
            }
            PackError::Decode(err) => {
                write!(f, "base64 decode err: {err}")
            }
        }
    }
}

impl From<std::str::Utf8Error> for PackError {
    fn from(err: std::str::Utf8Error) -> Self {
        PackError::Utf8(err)
    }
}

impl From<base64::DecodeError> for PackError {
    fn from(err: base64::DecodeError) -> Self {
        PackError::Decode(err)
    }
}

impl From<hex::FromHexError> for PackError {
    fn from(_err: hex::FromHexError) -> Self {
        PackError::FromHex
    }
}

impl std::error::Error for PackError {}
