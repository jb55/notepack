mod error;
mod note;
mod parser;
mod stringtype;
mod varint;

pub use error::Error;
pub use note::Note;
pub use parser::{NoteParser, ParsedField};
pub use stringtype::StringType;

use varint::{write_tagged_varint, write_varint};

pub fn pack_note(note: &Note) -> Result<Vec<u8>, Error> {
    let mut buf = Vec::new();

    // version
    write_varint(&mut buf, 1);

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

pub fn pack_note_to_string(note: &Note) -> Result<String, Error> {
    let bytes = pack_note(note)?;
    Ok(format!("notepack_{}", base64_encode(&bytes)))
}

fn base64_encode(bs: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

    STANDARD_NO_PAD.encode(bs)
}
