# notepack

**notepack** is a Rust library and CLI for encoding and decoding [nostr](https://github.com/nostr-protocol/nostr) notes into a **compact binary format**.

It ships with:

* 📦 **A Rust crate** — for embedding notepack logic into apps, relays, or tooling.
* 💻 **A CLI tool** — for piping JSON ↔ `notepack_…` strings in scripts.

---

## 🚀 What is notepack?

notepack is a **binary serialization** for Nostr “events” (`Note`), plus a recognizable string form.
It aims to:

* **Shrink payloads** using unsigned LEB128 (“varint”) integers.
* Store **common cryptographic fields** (id, pubkey, sig) as raw bytes.
* Preserve **UTF‑8** for text fields.
* Provide a **copy‑pasteable string** starting with `notepack_` + Base64 (RFC 4648, no padding).

📜 See [`SPEC.md`](SPEC.md) for the full format specification.

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

---

## ✨ Features

✅ **Compact:** Every integer is ULEB128 varint, tags are tagged‑varint.
✅ **Streaming parser:** No massive allocations; parse incrementally.
✅ **Safe round‑tripping:** Encode → decode → same note back.
✅ **CLI tool:** Turn JSON Nostr events into compact strings or back again.
✅ **Strict error handling:** Detects truncated data, overflow, bad UTF‑8, etc.

---

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

## 🔍 Spec Highlights

* **Fixed width** for `id`, `pubkey`, `sig` (32/32/64 bytes).
* **ULEB128 varints** for timestamps, lengths, etc.
* **Tagged‑varint** for tag elements (`is_bytes` vs `UTF‑8`).
* **String form**: `notepack_` + Base64 **no padding**.

See [SPEC.md](src/SPEC.md) for deep details, diagrams, and test vectors.

---

## 📜 License

MIT — do whatever you want, but attribution is appreciated.
