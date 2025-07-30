use crate::Error;
use crate::parser::read_string;
use crate::stringtype::StringType;
use crate::varint::{read_tagged_varint, read_varint};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NoteBuf {
    /// 32-bytes sha256 of the the serialized event data
    pub id: String,
    /// 32-bytes hex-encoded public key of the event creator
    pub pubkey: String,
    /// unix timestamp in seconds
    pub created_at: u64,
    /// integer
    /// 0: NostrEvent
    pub kind: u64,
    /// Tags
    pub tags: Vec<Vec<String>>,
    /// arbitrary string
    pub content: String,
    /// 64-bytes signature of the sha256 hash of the serialized event data, which is the same as the "id" field
    pub sig: String,
}

/// a Nostr note in notepack format
#[derive(Debug, Clone)]
pub struct Note<'a> {
    /// 32-bytes sha256 of the the serialized event data
    pub id: &'a [u8; 32],
    /// 32-bytes hex-encoded public key of the event creator
    pub pubkey: &'a [u8; 32],
    /// 64-bytes signature of the sha256 hash of the serialized event data, which is the same as the "id" field
    pub sig: &'a [u8; 64],
    /// arbitrary string
    pub content: &'a str,
    /// unix timestamp in seconds
    pub created_at: u64,
    /// integer
    /// 0: NostrEvent
    pub kind: u64,
    /// Tags
    pub tags: Tags<'a>,
}

impl<'a> Serialize for Note<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 7 fields per NIP-01: id, pubkey, created_at, kind, tags, content, sig
        let mut st = serializer.serialize_struct("Note", 7)?;

        // Hex-encode fixed-size fields (lowercase).
        st.serialize_field("id", &hex::encode(self.id))?;
        st.serialize_field("pubkey", &hex::encode(self.pubkey))?;
        st.serialize_field("created_at", &self.created_at)?;
        st.serialize_field("kind", &self.kind)?;

        // Materialize tags to Vec<Vec<String>> for JSON.
        // Strings pass through; raw bytes become lowercase hex strings.
        let mut tags_json: Vec<Vec<String>> = Vec::with_capacity(self.tags.len() as usize);
        let mut tags = self.tags.clone(); // don't mutate self

        while let Some(mut elems) = tags
            .next_tag()
            .map_err(|e| <S::Error as serde::ser::Error>::custom(e.to_string()))?
        {
            let mut tag_vec: Vec<String> = Vec::with_capacity(elems.remaining() as usize);
            while let Some(elem) = elems
                .next()
                .transpose()
                .map_err(|e| <S::Error as serde::ser::Error>::custom(e.to_string()))?
            {
                match elem {
                    crate::stringtype::StringType::Str(s) => tag_vec.push(s.to_string()),
                    crate::stringtype::StringType::Bytes(bs) => tag_vec.push(hex::encode(bs)),
                }
            }
            tags_json.push(tag_vec);
        }

        st.serialize_field("tags", &tags_json)?;
        st.serialize_field("content", &self.content)?;
        st.serialize_field("sig", &hex::encode(self.sig))?;

        st.end()
    }
}

/// A **lazy view** over tags in a packed [`Note`].
///
/// This is returned by [`NoteParser::into_note()`] or [`Tags::parse`].
/// It yields [`TagElems`] iterators—one for each tag block—without pre-scanning
/// or allocating. The underlying data is parsed lazily as you go.
///
/// Each tag is a sequence of elements (e.g. `["p", <pubkey>, "relay"]`), and
/// each element is either a UTF‑8 `str` or raw `&[u8]`, represented as [`StringType`].
///
/// # Example
///
/// ```rust
/// # use notepack::{NoteParser, StringType};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let packed = NoteParser::decode("notepack_737yskaxtaKQSL3IPPhOOR8T1R4G/f4ARPHGeNPfOpF4417q9YtU+4JZGOD3+Y0S3uVU6/edo64oTqJQ0pOF29Ms7GmX6fzM4Wjc6sohGPlbdRGLjhuqIRccETX5DliwUFy9qGg2lDD9oMl8ijoNFq4wwJ5Ikmr4Vh7NYWBwOkuo/anEBgECaGkA")?;
/// let note = NoteParser::new(&packed).into_note()?;
/// let mut tags = note.tags.clone();
///
/// while let Some(mut elems) = tags.next_tag()? {
///     for elem in &mut elems {
///         match elem? {
///             StringType::Str(s) => println!("str: {s}"),
///             StringType::Bytes(bs) => println!("hex: {}", hex::encode(bs)),
///         }
///     }
/// }
/// # Ok(()) }
/// ```
///
/// # Notes
///
/// - Dropping a [`TagElems`] early will fast-forward to the next tag automatically.
/// - Use [`TagElems::finish()`] to explicitly surface errors from any skipped elements.
#[derive(Debug, Clone)]
pub struct Tags<'a> {
    data: &'a [u8], // cursor: at the next tag's num_elems varint
    remaining: u64, // tags left
}

/// A lazy iterator over the elements of a single tag.
///
/// Yields each tag element as a [`StringType`] (either a UTF‑8 string or raw bytes),
/// parsed directly from the packed data.
///
/// This struct implements [`Iterator`].
///
/// # Notes
///
/// - Dropping a partially-consumed `TagElems` will fast-forward past remaining elements,
///   so the parent [`Tags`] iterator stays aligned on the next tag.
/// - If you want to catch errors in skipped elements (e.g. malformed UTF-8 or truncation),
///   use [`TagElems::finish()`].
///
/// # Example
///
/// ```rust
/// # use notepack::{NoteParser, StringType};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes = NoteParser::decode("notepack_737yskaxtaKQSL3IPPhOOR8T1R4G/f4ARPHGeNPfOpF4417q9YtU+4JZGOD3+Y0S3uVU6/edo64oTqJQ0pOF29Ms7GmX6fzM4Wjc6sohGPlbdRGLjhuqIRccETX5DliwUFy9qGg2lDD9oMl8ijoNFq4wwJ5Ikmr4Vh7NYWBwOkuo/anEBgECaGkA")?;
/// let note = NoteParser::new(&bytes).into_note()?;
/// let mut tags = note.tags.clone();
///
/// while let Some(mut elems) = tags.next_tag()? {
///     for elem in &mut elems {
///         match elem? {
///             StringType::Str(s) => println!("text: {s}"),
///             StringType::Bytes(bs) => println!("hex: {}", hex::encode(bs)),
///         }
///     }
/// }
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct TagElems<'a, 'p> {
    cursor: &'p mut &'a [u8], // shared cursor with parent
    remaining: u64,           // elements left in this tag
}

impl<'a> Tags<'a> {
    /// Parse the tags block at the current cursor.
    ///
    /// `input` must point to the varint `num_tags` (the start of the tags block).
    /// On success, this consumes that varint and returns a cursor positioned at the
    /// first tag’s `num_elems`.
    pub fn parse(input: &mut &'a [u8]) -> Result<Self, Error> {
        let num_tags = read_varint(input)?;
        Ok(Self {
            data: *input,
            remaining: num_tags,
        })
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.remaining
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Lazily advance to the next tag and return an iterator over its elements.
    ///
    /// This reads only the tag’s `num_elems` varint; element payloads are consumed
    /// by the returned `TagElems`. If you drop `TagElems` early, it will fast‑forward
    /// the remaining elements so the parent cursor lands at the next tag.
    pub fn next_tag<'p>(&'p mut self) -> Result<Option<TagElems<'a, 'p>>, Error> {
        if self.remaining == 0 {
            return Ok(None);
        }
        // Read this tag's num_elems; leave cursor at the first element.
        let num_elems = read_varint(&mut self.data)?;
        self.remaining -= 1;
        Ok(Some(TagElems {
            cursor: &mut self.data,
            remaining: num_elems,
        }))
    }
}

impl<'a, 'p> TagElems<'a, 'p> {
    #[inline]
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Explicitly finish (skip any remaining elements).
    /// Prefer this if you want errors surfaced instead of silent best‑effort in Drop.
    pub fn finish(mut self) -> Result<(), Error> {
        while self.remaining > 0 {
            let (len, _is_bytes) = read_tagged_varint(self.cursor)?;
            if self.cursor.len() < len as usize {
                return Err(Error::Truncated);
            }
            *self.cursor = &self.cursor[len as usize..];
            self.remaining -= 1;
        }
        Ok(())
    }
}

impl<'a, 'p> Iterator for TagElems<'a, 'p> {
    type Item = Result<StringType<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        // Read one tagged string and advance the shared cursor.
        let item = read_string(self.cursor);
        match item {
            Ok(s) => {
                self.remaining -= 1;
                Some(Ok(s))
            }
            Err(e) => {
                // Poison the iterator; parent cursor is wherever the error occurred.
                self.remaining = 0;
                Some(Err(e))
            }
        }
    }
}

/// Best‑effort fast‑forward on early drop so parent cursor lands at the next tag.
/// Errors can’t be propagated in Drop, so this intentionally ignores them.
/// If robust error handling matters, call `finish()` instead.
impl<'a, 'p> Drop for TagElems<'a, 'p> {
    fn drop(&mut self) {
        // If fully drained, do nothing.
        while self.remaining > 0 {
            if let Ok((len, _is_bytes)) = read_tagged_varint(self.cursor) {
                if self.cursor.len() < len as usize {
                    break; // truncated; leave cursor as-is
                }
                *self.cursor = &self.cursor[len as usize..];
                self.remaining -= 1;
            } else {
                break; // malformed; leave cursor as-is
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::varint::{write_tagged_varint, write_varint};

    fn push_elem_str(buf: &mut Vec<u8>, s: &str) {
        write_tagged_varint(buf, s.len() as u64, false);
        buf.extend_from_slice(s.as_bytes());
    }

    fn push_elem_bytes(buf: &mut Vec<u8>, bs: &[u8]) {
        write_tagged_varint(buf, bs.len() as u64, true);
        buf.extend_from_slice(bs);
    }

    /// Build a tags block:
    /// [varint num_tags] [varint num_elems][elem..] ... repeated
    fn build_tags_block(tags: &[Vec<ElemSpec>]) -> Vec<u8> {
        let mut buf = Vec::new();
        write_varint(&mut buf, tags.len() as u64);
        for tag in tags {
            write_varint(&mut buf, tag.len() as u64);
            for e in tag {
                match e {
                    ElemSpec::Str(s) => push_elem_str(&mut buf, s),
                    ElemSpec::Bytes(bs) => push_elem_bytes(&mut buf, bs),
                }
            }
        }
        buf
    }

    #[derive(Debug, Clone)]
    enum ElemSpec {
        Str(&'static str),
        Bytes(&'static [u8]),
    }

    #[test]
    fn tags_iterates_all_elements_correctly() -> Result<(), Error> {
        // tag0: ["p", 0xaabb (bytes), "hello"]
        // tag1: [""]
        let block = build_tags_block(&[
            vec![
                ElemSpec::Str("p"),
                ElemSpec::Bytes(&[0xaa, 0xbb]),
                ElemSpec::Str("hello"),
            ],
            vec![ElemSpec::Str("")],
        ]);

        let mut input = block.as_slice();

        // parse the block into a lazy Tags cursor
        let mut tags = Tags::parse(&mut input)?;
        assert_eq!(tags.len(), 2);
        assert!(!tags.is_empty());

        // tag 0
        {
            let mut t0 = tags.next_tag()?.expect("tag0");
            let mut out = Vec::new();
            while let Some(x) = t0.next() {
                match x? {
                    StringType::Str(s) => out.push(format!("S:{s}")),
                    StringType::Bytes(bs) => out.push(format!("B:{}", hex::encode(bs))),
                }
            }
            assert_eq!(out, &["S:p", "B:aabb", "S:hello"]);
        }

        // tag 1
        {
            let mut t1 = tags.next_tag()?.expect("tag1");
            let got = t1.next().expect("1 elem")?;
            match got {
                StringType::Str(s) => assert_eq!(s, ""),
                _ => panic!("expected empty string"),
            }
            assert!(t1.next().is_none());
        }

        // done
        assert!(tags.next_tag()?.is_none());
        Ok(())
    }

    #[test]
    fn dropping_tag_elems_early_fast_forwards_to_next_tag() -> Result<(), Error> {
        // tag0: ["a","b","c"] — we'll consume only the first elem then drop
        // tag1: ["z"]
        let block = build_tags_block(&[
            vec![ElemSpec::Str("a"), ElemSpec::Str("b"), ElemSpec::Str("c")],
            vec![ElemSpec::Str("z")],
        ]);

        let mut input = block.as_slice();
        let mut tags = Tags::parse(&mut input)?;

        // tag 0: consume one element and drop early
        {
            let mut t0 = tags.next_tag()?.expect("tag0");
            let first = t0.next().expect("has first")?;
            match first {
                StringType::Str("a") => {}
                _ => panic!("unexpected first element"),
            }
            // t0 dropped here with remaining=2; Drop should skip "b","c"
        }

        // We should now be aligned at tag1
        {
            let mut t1 = tags.next_tag()?.expect("tag1");
            let first = t1.next().expect("has z")?;
            match first {
                StringType::Str("z") => {}
                _ => panic!("expected 'z'"),
            }
            assert!(t1.next().is_none());
        }

        // No more tags
        assert!(tags.next_tag()?.is_none());
        Ok(())
    }

    #[test]
    fn finish_reports_truncation_error() {
        // Build a malformed tag:
        // num_tags=1, tag0 num_elems=1, element claims len=10 but provides only 3 bytes
        let mut buf = Vec::new();
        write_varint(&mut buf, 1); // one tag
        write_varint(&mut buf, 1); // one element
        write_tagged_varint(&mut buf, 10, false); // claim 10-byte UTF-8
        buf.extend_from_slice(b"abc"); // only 3 bytes -> truncated

        let mut input = buf.as_slice();
        let mut tags = Tags::parse(&mut input).expect("parse ok");
        let elems = tags.next_tag().expect("ok").expect("tag");

        // Using finish() should surface the error
        let err = elems.finish().unwrap_err();
        match err {
            Error::Truncated => {} // expected
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
