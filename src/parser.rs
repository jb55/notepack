use crate::error::Error;
use crate::stringtype::StringType;
use crate::varint::{read_tagged_varint, read_varint};

#[derive(Debug, Clone)]
pub enum ParsedField<'a> {
    Version(u8),
    Id(&'a [u8]),
    Pubkey(&'a [u8]),
    Sig(&'a [u8]),
    CreatedAt(u64),
    Kind(u64),
    Content(&'a str),
    NumTags(u64),
    NumTagElems(u64),
    Tag(StringType<'a>),
}

#[derive(Debug, Clone)]
pub struct NoteParser<'a> {
    data: &'a [u8],
    state: ParserState,
    tags_remaining: u64,
    elems_remaining: u64,
}

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
    fn is_halted(self) -> bool {
        self == Self::Done || self == Self::Errored
    }
}

impl<'a> NoteParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            state: ParserState::Start,
            tags_remaining: 0,
            elems_remaining: 0,
        }
    }

    /// Start parsing a notepack_... string
    pub fn decode(notepack: &'a str) -> Result<Vec<u8>, Error> {
        if let Some(b64) = notepack.strip_prefix("notepack_") {
            Ok(base64_decode(b64)?)
        } else {
            Err(Error::InvalidPrefix)
        }
    }

    pub fn current_state(&self) -> ParserState {
        self.state
    }
}

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

    STANDARD_NO_PAD.decode(s)
}

impl<'a> Iterator for NoteParser<'a> {
    type Item = Result<ParsedField<'a>, Error>;

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
                let s = read_or_err!(std::str::from_utf8(bytes).map_err(|e| Error::Utf8(e)));
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

pub fn read_bytes<'a>(len: u64, input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    let (head, tail) = input.split_at(len as usize);
    *input = tail;
    Ok(head)
}

pub fn read_string<'a>(input: &mut &'a [u8]) -> Result<StringType<'a>, Error> {
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
