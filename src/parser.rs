use crate::error::Error;
use crate::stringtype::StringType;
use crate::varint::{read_tagged_varint, read_varint};

/// Represents a parsed field from a notepack‐encoded Nostr note.
///
/// Each variant corresponds to a logical field in the binary format,
/// emitted sequentially by the [`NoteParser`] iterator as it reads
/// through the byte stream.
#[derive(Debug, Clone)]
pub enum ParsedField<'a> {
    /// Format version (currently always `1`).
    Version(u8),

    /// 32‑byte event ID (SHA‑256 of serialized event).
    Id(&'a [u8]),

    /// 32‑byte secp256k1 public key of the author.
    Pubkey(&'a [u8]),

    /// 64‑byte Schnorr signature of the event ID.
    Sig(&'a [u8]),

    /// Unix timestamp (seconds) of event creation.
    CreatedAt(u64),

    /// Event kind (u64 varint).
    Kind(u64),

    /// UTF‑8 encoded event body.
    Content(&'a str),

    /// Number of tags present (varint).
    NumTags(u64),

    /// Number of elements in the next tag (varint).
    NumTagElems(u64),

    /// A single tag element: either [`StringType::Str`] or [`StringType::Bytes`].
    Tag(StringType<'a>),
}

/// Stateful streaming parser for notepack binary payloads.
///
/// Yields [`ParsedField`] items in the order they appear in the binary format.
/// Errors are non‑recoverable: once an error is yielded, the parser halts.
///
/// Implements [`Iterator`], so you can do:
///
/// ```rust
/// # use notepack::{NoteParser, ParsedField};
/// if let Ok(bytes) = NoteParser::decode("notepack_Hq7oszfVbWy7ZF...") {
///     let parser = NoteParser::new(&bytes);
///     for field in parser {
///         println!("{:?}", field);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct NoteParser<'a> {
    /// Remaining bytes to parse.
    data: &'a [u8],

    /// Current parsing state machine position.
    state: ParserState,

    /// Number of tags left to read.
    tags_remaining: u64,

    /// Number of elements remaining in the current tag.
    elems_remaining: u64,
}

/// Internal parser state machine.
///
/// Parsing transitions linearly (Start → AfterVersion → … → Done).
/// Once in [`ParserState::Errored`] or [`ParserState::Done`], the parser ha
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParserState {
    Start,
    AfterVersion,
    AfterId,
    AfterPubkey,
    AfterSig,
    AfterCreatedAt,
    AfterKind,
    AfterContent,
    ReadingTags,
    Done,
    Errored,
}

impl ParserState {
    /// Returns `true` if no more fields will be produced (`Done` or `Errored`).
    fn is_halted(self) -> bool {
        self == Self::Done || self == Self::Errored
    }
}

impl<'a> NoteParser<'a> {
    /// Create a new [`NoteParser`] over a binary notepack slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            state: ParserState::Start,
            tags_remaining: 0,
            elems_remaining: 0,
        }
    }

    /// Decode a `notepack_...` Base64 string into raw bytes.
    ///
    /// Strips the `"notepack_"` prefix and base64‑decodes the remainder.
    /// Returns [`Error::InvalidPrefix`] if the string does not start with
    pub fn decode(notepack: &'a str) -> Result<Vec<u8>, Error> {
        if let Some(b64) = notepack.strip_prefix("notepack_") {
            Ok(base64_decode(b64)?)
        } else {
            Err(Error::InvalidPrefix)
        }
    }

    /// Return the current [`ParserState`] (mainly for debugging or inspection).
    pub fn current_state(&self) -> ParserState {
        self.state
    }
}

/// Base64 decode using the RFC 4648 alphabet **without padding** (`=`).
fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

    STANDARD_NO_PAD.decode(s)
}

impl<'a> Iterator for NoteParser<'a> {
    type Item = Result<ParsedField<'a>, Error>;

    /// Parse the next [`ParsedField`] from the input buffer.
    ///
    /// Returns `None` when parsing is complete or after an unrecoverable error.
    fn next(&mut self) -> Option<Self::Item> {
        use ParserState::*;

        if self.state.is_halted() {
            return None;
        }

        // small helper to make error propagation less noisy
        macro_rules! read_or_err {
            ($expr:expr) => {
                match $expr {
                    Ok(val) => val,
                    Err(e) => {
                        self.state = Errored;
                        return Some(Err(e));
                    }
                }
            };
        }

        let item = match self.state {
            Start => {
                let version = read_or_err!(read_varint(&mut self.data)) as u8;
                self.state = AfterVersion;
                Ok(ParsedField::Version(version))
            }
            AfterVersion => {
                let id = read_or_err!(read_bytes(32, &mut self.data));
                self.state = AfterId;
                Ok(ParsedField::Id(id))
            }
            AfterId => {
                let pk = read_or_err!(read_bytes(32, &mut self.data));
                self.state = AfterPubkey;
                Ok(ParsedField::Pubkey(pk))
            }
            AfterPubkey => {
                let sig = read_or_err!(read_bytes(64, &mut self.data));
                self.state = AfterSig;
                Ok(ParsedField::Sig(sig))
            }
            AfterSig => {
                let ts = read_or_err!(read_varint(&mut self.data));
                self.state = AfterCreatedAt;
                Ok(ParsedField::CreatedAt(ts))
            }
            AfterCreatedAt => {
                let kind = read_or_err!(read_varint(&mut self.data));
                self.state = AfterKind;
                Ok(ParsedField::Kind(kind))
            }
            AfterKind => {
                let content_len = read_or_err!(read_varint(&mut self.data));
                let bytes = read_or_err!(read_bytes(content_len, &mut self.data));
                let s = read_or_err!(std::str::from_utf8(bytes).map_err(Error::Utf8));
                self.state = AfterContent;
                Ok(ParsedField::Content(s))
            }
            AfterContent => {
                let num_tags = read_or_err!(read_varint(&mut self.data));
                self.tags_remaining = num_tags;
                self.state = if num_tags > 0 { ReadingTags } else { Done };
                Ok(ParsedField::NumTags(num_tags))
            }
            ReadingTags => {
                if self.elems_remaining == 0 {
                    if self.tags_remaining == 0 {
                        self.state = Done;
                        return None;
                    }
                    let num_elems = read_or_err!(read_varint(&mut self.data));
                    self.elems_remaining = num_elems;
                    self.tags_remaining -= 1;
                    return Some(Ok(ParsedField::NumTagElems(num_elems)));
                }

                let tag = read_or_err!(read_string(&mut self.data));
                self.elems_remaining -= 1;
                Ok(ParsedField::Tag(tag))
            }
            Done => return None,
            Errored => return None,
        };

        Some(item)
    }
}

/// Read exactly `len` bytes from the input slice.
///
/// Returns [`Error::Truncated`] if fewer than `len` bytes remain.
fn read_bytes<'a>(len: u64, input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    let (head, tail) = input.split_at(len as usize);
    *input = tail;
    Ok(head)
}

/// Read a tagged string (see §3.2 of spec) from the input.
///
/// Uses [`read_tagged_varint`] to determine payload length and type.
/// Returns:
///  * [`StringType::Str`] if `is_bytes == false`
///  * [`StringType::Bytes`] if `is_bytes == true`
fn read_string<'a>(input: &mut &'a [u8]) -> Result<StringType<'a>, Error> {
    let (len, is_bytes) = read_tagged_varint(input)?;
    if input.len() < len as usize {
        return Err(Error::Truncated);
    }
    let (head, tail) = input.split_at(len as usize);
    *input = tail;

    Ok(if is_bytes {
        StringType::Bytes(head)
    } else {
        StringType::Str(std::str::from_utf8(head)?)
    })
}
