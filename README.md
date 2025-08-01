# notepack

[![Docs.rs](https://docs.rs/notepack/badge.svg)](https://docs.rs/notepack) [![Crates.io](https://img.shields.io/crates/v/notepack.svg)](https://crates.io/crates/notepack)

**notepack** is a **compact binary note** format for [nostr](https://github.com/nostr-protocol/nostr) [notes][nip01], with a [specification](./SPEC.md) and reference implementation in Rust.

It ships with:

* 📦 **A Rust crate** — for embedding notepack logic into apps, relays, or tooling.
* 💻 **A CLI tool** — for piping JSON ↔ `notepack_…` strings in scripts.

📜 See [`SPEC.md`](SPEC.md) for the full format specification.

---

## ✨ Features

* **Copy‑pasteable string** starting with `notepack_` + Base64 (RFC 4648, no padding).
* **Compact:** Every integer is ULEB128 varint. hex strings in tags are **encoded as bytes**.
* **50% size reduction:** Many large events like contact lists see a 50% reduction in size
* **Simple:** So simple its a candidate for nostr's canonical binary representation

---

## Benchmarks

```
$ cargo bench
```

Contact list note with 1022 tags:

* **notepack-decode**: 2GB/s, 15 microseconds
* **json-decode**: 711MB/s, 100 microseconds

The numbers start to count when you are decoding lots of notes: 1000 notes would take 100ms with json, 15ms with notepack.

If not iterating tags, notepack gets up to 1TB/s at 30 nanoseconds. That's only 0.03ms for 1000 notes if you're not verifying and just want to check a few fields as they stream in.

...but didn't feel like that was a fair comparison since you'll likely need to iterate the tags to verify the note.

I have lots of hacks in [nostrdb][nostrdb] to do incremental json parsing for note de-duplication/note rejection. with notepack you can get the ID and skip it in less than 15 microseconds.


## Example

```
$ notepack <<<'{"id": "f1e7bc2a9756453fcc0e80ecf62183fa95b9a1278a01281dbc310b6777320e80","pubkey": "7fe437db5884ee013f701a75f8d1a84ecb434e997f2a31411685551ffff1b841","created_at": 1753900182,"kind": 1,"tags": [],"content": "hi","sig": "75507f84d78211a68f2f964221f5587aa957a66c1941d01125caa07b9aabdf5a98c3e63d1fe1e307cbf01b74b0a1b95ffe636eb6746c00167e0d48e5b11032d5"}'

notepack_AfHnvCqXVkU/zA6A7PYhg/qVuaEnigEoHbwxC2d3Mg6Af+Q321iE7gE/cBp1+NGoTstDTpl/KjFBFoVVH//xuEF1UH+E14IRpo8vlkIh9Vh6qVembBlB0BElyqB7mqvfWpjD5j0f4eMHy/AbdLChuV/+Y262dGwAFn4NSOWxEDLVlsmpxAYBAmhpAA
```

* json string: 363 bytes
* notepack string: 124 bytes raw, 196 base64-encoded

For large contact lists, you can crunch them down from 74kb to about 36kb.

## 📦 Usage (Library)

### Encoding

```rust
use notepack::{Note, pack_note_to_string};

let note = Note {
    id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    pubkey: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
    created_at: 1753898766,
    kind: 1,
    tags: vec![vec!["tag".into(), "value".into()]],
    content: "Hello, world!".into(),
    sig: "cccc...".into(),
};

let encoded = pack_note_to_string(&note).unwrap();
println!("{encoded}"); // => notepack_AAECAw...
```

### Streaming Decode

```rust
use notepack::{NoteParser, ParsedField};

let b64 = "notepack_..."; // from wire
let bytes = NoteParser::decode(b64).unwrap();
let parser = NoteParser::new(&bytes);

for field in parser {
    match field.unwrap() {
        ParsedField::Id(id) => println!("id: {}", hex::encode(id)),
        ParsedField::Content(c) => println!("content: {}", c),
        _ => {}
    }
}
```

---

## 💻 CLI Usage

The binary is also called `notepack`.

### Encode JSON → notepack string

```bash
echo '{"id":"...","pubkey":"...","created_at":123,"kind":1,"tags":[],"content":"Hi","sig":"..."}' \
  | notepack
```

### Decode notepack string → JSON

```bash
echo 'notepack_AAECA...' | notepack
```

---

## 📂 Project Structure

```
src
├── SPEC.md         # Full binary format spec
├── error.rs        # Unified error type for encoding/decoding
├── lib.rs          # Crate entrypoint
├── main.rs         # CLI tool: JSON ↔ notepack
├── note.rs         # `Note` struct (Nostr event model)
├── parser.rs       # Streaming `NoteParser`
├── stringtype.rs   # String vs raw byte tags
└── varint.rs       # LEB128 varint helpers
```

## 📜 License

MIT — do whatever you want, but attribution is appreciated.

[nostrdb]: https://github.com/damus-io/nostrdb
[nip01]: https://github.com/nostr-protocol/nips/blob/master/01.md#events-and-signatures
