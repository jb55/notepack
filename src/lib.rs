mod error;
mod note;
mod varint;

use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
pub use error::PackError;
pub use note::Note;
use varint::{read_tagged_varint, read_varint, write_tagged_varint, write_varint};

pub enum StringType<'a> {
    Bytes(&'a [u8]),
    Str(&'a str),
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
            write_string(&mut buf, elem);
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
    let (len, is_bytes) = read_tagged_varint(input)?;
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
    Ok(format!("notepack_{}", base64_encode(&bytes)))
}

fn base64_encode(bs: &[u8]) -> String {
    STANDARD_NO_PAD.encode(bs)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    STANDARD_NO_PAD.decode(s)
}

pub fn unpack_note_from_string(string: &str) -> Result<String, PackError> {
    let bytes = base64_decode(string)?;
    let mut data: &[u8] = &bytes;

    let id = read_bytes(32, &mut data)?;
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
        eprintln!();
    }
    eprintln!();

    Ok(hex::encode(bytes))
    //Ok("".to_string())
}
