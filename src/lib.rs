//! # notepack
//!
//! A Rust library for packing and parsing [nostr](https://github.com/nostr-protocol/nostr) notes
//! into a compact binary format called **notepack**.  
//!
//! This crate provides two core capabilities:
//!
//! - **Encoding**: Turn a [`Note`] (a structured Nostr event) into a notepack binary, or a Base64
//!   string prefixed with `notepack_`.
//! - **Decoding / Streaming Parsing**: Efficiently stream through a binary notepack payload using
//!   [`NoteParser`], yielding fields as they are parsed (without needing to fully deserialize).
//!
//! ## Features
//!
//! - **Compact binary format** using varint encoding for integers.
//! - **Streaming parser**: no allocation-heavy parsing; fields are yielded one by one as they’re read.
//!
//! ## Example: Encoding a Note
//!
//! ```rust
//! use notepack::{Note, pack_note_to_string};
//!
//! let note = Note {
//!     id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
//!     pubkey: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
//!     created_at: 1753898766,
//!     kind: 1,
//!     tags: vec![vec!["tag".into(), "value".into()]],
//!     content: "Hello, world!".into(),
//!     sig: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
//! };
//!
//! let packed = pack_note_to_string(&note).unwrap();
//! println!("{packed}");
//! // prints something like `notepack_AAECA...`
//! ```
//!
//! ## Example: Streaming Parse
//!
//! ```rust
//! use notepack::{NoteParser, ParsedField};
//!
//! let b64 = "notepack_737yskaxtaKQSL3IPPhOOR8T1R4G/f4ARPHGeNPfOpF4417q9YtU+4JZGOD3+Y0S3uVU6/edo64oTqJQ0pOF29Ms7GmX6fzM4Wjc6sohGPlbdRGLjhuqIRccETX5DliwUFy9qGg2lDD9oMl8ijoNFq4wwJ5Ikmr4Vh7NYWBwOkuo/anEBgECaGkA";
//! let bytes = NoteParser::decode(b64).unwrap();
//! let parser = NoteParser::new(&bytes);
//!
//! for field in parser {
//!     match field.unwrap() {
//!         ParsedField::Id(id) => println!("id: {}", hex::encode(id)),
//!         ParsedField::Content(c) => println!("content: {}", c),
//!         _ => {}
//!     }
//! }
//! ```
//!
//! ## Binary Tool
//!
//! This crate also ships with a small CLI called `notepack` (see `main.rs`):
//!
//! - **Pipe in a JSON Nostr event** → outputs a `notepack_...` string.
//! - **Pipe in a `notepack_...` string** → outputs the JSON representation.
//!
//! ```bash
//! echo '{"id":"...","pubkey":"...","created_at":123,"kind":1,"tags":[],"content":"Hi","sig":"..."}' \
//!   | notepack
//! ```
//!
//! ## Modules
//!
//! - [`Note`] — main event struct used for encoding.
//! - [`NoteParser`] — streaming parser for notepack binaries.
//! - [`ParsedField`] — enum of parsed fields yielded by the parser.
//! - [`Error`] — unified error type.
//! - [`StringType`] — distinguishes between raw byte tags and UTF-8 tags.
//!
//! ## Spec
//!
//! The notepack format is loosely inspired by [MessagePack](https://msgpack.org/) but optimized for
//! Nostr notes. Strings that look like 32-byte hex are stored more compactly; integers are encoded
//! as LEB128-style varints; and the format starts with a `version` field for forward compatibility.

mod error;
mod note;
mod parser;
mod stringtype;
mod varint;

pub use error::Error;
pub use note::Note;
pub use parser::{NoteParser, ParsedField, ParserState};
pub use stringtype::StringType;

use varint::{write_tagged_varint, write_varint};

/// Packs a [`Note`] into its compact binary notepack representation.
///
/// This function serializes a [`Note`] into the raw notepack binary format:
/// - Adds version (currently `1`) as a varint.
/// - Encodes fixed-size fields (`id`, `pubkey`, `sig`) as raw bytes.
/// - Writes variable-length fields (`content`, `tags`) with varint length prefixes.
/// - Optimizes strings that look like 32-byte hex by storing them in a compressed form.
///
/// Returns a `Vec<u8>` containing the binary payload, or an [`Error`] if hex decoding fails.
///
/// This is the low-level encoding API—most callers will want [`pack_note_to_string`] instead.
///
/// # Errors
///
/// Returns [`Error::Hex`] if any hex string field (like `id`, `pubkey`, or `sig`) fails to decode.
///
/// # Example
///
/// ```rust
/// use notepack::{Note, pack_note};
///
/// let note = Note::default();
/// let binary = pack_note(&note).unwrap();
/// assert!(binary.len() > 0);
/// ```
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

/// Encodes a [`Note`] directly to a `notepack_...` Base64 string.
///
/// This is a convenience wrapper around [`pack_note`], taking the binary payload and
/// Base64-encoding it (without padding) and prefixing with `notepack_`.
///
/// This is the primary API for exporting notes for storage, transmission, or embedding in JSON.
///
/// # Errors
///
/// Returns the same [`Error`]s as [`pack_note`], e.g. hex decoding issues.
///
/// # Example
///
/// ```rust
/// use notepack::{Note, pack_note_to_string};
///
/// let note = Note::default();
/// let s = pack_note_to_string(&note).unwrap();
/// assert!(s.starts_with("notepack_"));
/// ```
pub fn pack_note_to_string(note: &Note) -> Result<String, Error> {
    let bytes = pack_note(note)?;
    Ok(format!("notepack_{}", base64_encode(&bytes)))
}

fn base64_encode(bs: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

    STANDARD_NO_PAD.encode(bs)
}

/// Only lower cased hex are allowed, otherwise encoding
/// wouldn't round-trip
fn decode_lowercase_hex(input: &str) -> Result<Vec<u8>, Error> {
    // Reject uppercase hex
    if input.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(Error::FromHex);
    }

    // Reject odd-length hex strings
    if input.len() % 2 != 0 {
        return Err(Error::FromHex);
    }

    Ok(hex::decode(input)?)
}

fn write_string(buf: &mut Vec<u8>, string: &str) {
    // we check to see if the entire string is 32-byte-hex
    if string.is_empty() {
        write_tagged_varint(buf, 0, false);
        return;
    }

    if let Ok(val) = decode_lowercase_hex(string) {
        write_tagged_varint(buf, val.len() as u64, true);
        buf.extend_from_slice(&val);
    } else {
        write_tagged_varint(buf, string.len() as u64, false);
        buf.extend_from_slice(string.as_bytes());
    }
}
