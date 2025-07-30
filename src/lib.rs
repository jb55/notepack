mod base62;
mod note;

use base62::{base62_decode, base62_encode};
pub use note::Note;

pub enum StringType<'a> {
    Bytes(&'a [u8]),
    Str(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    Truncated,
    VarintOverflow,
    VarintUnterminated,
    Utf8(std::str::Utf8Error),
    FromHex,
    Decode,
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
            PackError::Decode => {
                write!(f, "base62 decode err")
            }
        }
    }
}

impl From<std::str::Utf8Error> for PackError {
    fn from(err: std::str::Utf8Error) -> Self {
        PackError::Utf8(err)
    }
}

impl From<hex::FromHexError> for PackError {
    fn from(_err: hex::FromHexError) -> Self {
        PackError::FromHex
    }
}

impl From<base62::DecodeError> for PackError {
    fn from(_err: base62::DecodeError) -> Self {
        PackError::Decode
    }
}

impl std::error::Error for PackError {}

fn write_varint(buf: &mut Vec<u8>, mut n: u64) -> usize {
    let mut len = 0;
    loop {
        let mut b = (n & 0x7F) as u8; // low 7 bits
        n >>= 7;
        if n != 0 {
            b |= 0x80; // continuation
        }
        buf.push(b);
        len += 1;
        if n == 0 {
            break;
        }
    }
    len
}

pub fn write_tagged_varint(buf: &mut Vec<u8>, value: u64, tagged: bool) -> usize {
    let tagged = value
        .checked_shl(1)
        .expect("value too large for tagged varint")
        | (tagged as u64);
    write_varint(buf, tagged)
}

pub fn read_varint(input: &mut &[u8]) -> Result<u64, PackError> {
    let mut n = 0u64;
    let mut shift = 0u32;

    for i in 0..input.len() {
        let b = input[i];
        let chunk = (b & 0x7F) as u64;
        n |= chunk << shift;

        if b & 0x80 == 0 {
            *input = &input[i + 1..]; // advance the slice handle
            return Ok(n);
        }

        shift += 7;
        if shift >= 64 {
            return Err(PackError::VarintOverflow);
        }
    }
    Err(PackError::VarintUnterminated)
}

pub fn read_tagged_varint_adv(input: &mut &[u8]) -> Result<(u64, bool), PackError> {
    let raw = read_varint(input)?;
    Ok((raw >> 1, (raw & 1) != 0))
}

pub fn pack_note(note: &Note) -> Result<Vec<u8>, PackError> {
    let mut buf = Vec::new();

    // id
    let id_bytes = hex::decode(&note.id)?;
    buf.extend_from_slice(&id_bytes);

    // pubkey
    let pk_bytes = hex::decode(&note.pubkey)?;
    buf.extend_from_slice(&pk_bytes);

    // signature
    let sig_bytes = hex::decode(&note.sig)?;
    buf.extend_from_slice(&sig_bytes);

    write_varint(&mut buf, note.created_at);
    write_varint(&mut buf, note.kind);
    write_varint(&mut buf, note.content.len() as u64);
    buf.extend_from_slice(note.content.as_bytes());

    write_varint(&mut buf, note.tags.len() as u64);
    for tag in &note.tags {
        write_varint(&mut buf, tag.len() as u64);

        for elem in tag {
            write_string(&mut buf, &elem);
        }
    }

    Ok(buf)
}

pub fn write_string(buf: &mut Vec<u8>, string: &str) {
    // we check to see if the entire string is 32-byte-hex
    if string.is_empty() {
        write_tagged_varint(buf, 0, false);
        return;
    }

    if let Ok(val) = hex::decode(string) {
        write_tagged_varint(buf, val.len() as u64, true);
        buf.extend_from_slice(&val);
    } else {
        write_tagged_varint(buf, string.len() as u64, false);
        buf.extend_from_slice(string.as_bytes());
    }
}

pub fn read_bytes<'a>(len: u64, input: &mut &'a [u8]) -> Result<&'a [u8], PackError> {
    let (head, tail) = input.split_at(len as usize);
    *input = tail;
    Ok(head)
}

pub fn read_string<'a>(input: &mut &'a [u8]) -> Result<StringType<'a>, PackError> {
    let (len, is_bytes) = read_tagged_varint_adv(input)?;
    if input.len() < len as usize {
        return Err(PackError::Truncated);
    }
    let (head, tail) = input.split_at(len as usize);
    *input = tail;

    Ok(if is_bytes {
        StringType::Bytes(head)
    } else {
        StringType::Str(std::str::from_utf8(head)?)
    })
}

pub fn pack_note_to_string(note: &Note) -> Result<String, PackError> {
    let bytes = pack_note(note)?;
    Ok(format!("notepack_{}", base62_encode(&bytes)))
}

pub fn unpack_note_from_string(string: &str) -> Result<String, PackError> {
    let bytes = base62_decode(string)?;
    let mut data: &[u8] = &bytes;

    let id = read_bytes(32, &mut data)?;
    eprintln!("");
    eprintln!("id:{}", hex::encode(id));

    let pk = read_bytes(32, &mut data)?;
    eprintln!("pk:{}", hex::encode(pk));

    let sig = read_bytes(64, &mut data)?;
    eprintln!("sig:{}", hex::encode(sig));

    let created_at = read_varint(&mut data)?;
    eprintln!("created_at:{}", created_at);

    let kind = read_varint(&mut data)?;
    eprintln!("kind:{}", kind);

    let content_len = read_varint(&mut data)?;
    let content = std::str::from_utf8(read_bytes(content_len, &mut data)?)?;
    eprintln!("content:'{}'", content);

    let num_tags = read_varint(&mut data)?;
    for _tag in 0..num_tags {
        let num_elems = read_varint(&mut data)?;

        for elem in 0..num_elems {
            if elem != 0 {
                eprint!(",");
            }
            match read_string(&mut data)? {
                StringType::Bytes(bs) => {
                    eprint!("b:{}", hex::encode(bs));
                }
                StringType::Str(s) => {
                    eprint!("s:{s}");
                }
            }
        }
        eprintln!("");
    }

    Ok(hex::encode(bytes))
    //Ok("".to_string())
}
