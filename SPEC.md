# notepack — A compact binary format for Nostr notes

**Version:** 0.1 (draft) — **Date:** July 30, 2025
**Status:** Experimental

## 1. Overview

**notepack** is a compact binary serialization of a Nostr “event” (`Note`) plus a string form for copy‑paste. It is designed to:

* Minimize size using unsigned LEB128 (“varint”) integers.
* Encode common 32/64‑byte hex values (ids, pubkeys, sigs, tag payloads) as raw bytes.
* Preserve UTF‑8 for human‑readable fields.
* Offer a recognizable string form prefixed with `notepack_` and Base64 (RFC 4648) without padding.

The format is deliberately simple: fixed‑width binary for the three cryptographic fields, varints for everything count/length‑like, and a **tagged‑varint** for each tag element indicating whether its payload is raw bytes or UTF‑8.

---

## 2. Data model

* See [NIP01][nip01] note encoding

---

## 3. Binary encoding

All multi‑byte integers use **unsigned LEB128 (ULEB128)**. Bit 7 indicates continuation; bits 0–6 carry payload; least‑significant group first.

### 3.1 Top‑level layout

```
notepack-binary = 
    version                ; u8 version tag
  | id[32]                 ; raw 32 bytes
  | pubkey[32]             ; raw 32 bytes
  | sig[64]                ; raw 64 bytes
  | varint(created_at)     ; u64 LEB128
  | varint(kind)           ; u64 LEB128
  | varint(content_len)    ; byte length of content
  | content[content_len]   ; UTF-8 bytes (no tag)
  | varint(num_tags)
  | repeated num_tags * tag
```

### 3.2 Tags

```
tag = 
  varint(num_elems)
  repeated num_elems * tag_elem
```

Each **tag\_elem** is length‑prefixed with a **tagged‑varint**:

```
tag_elem =
  tagged_varint(len, is_bytes)
  | payload[len]
```

* `tagged_varint` packs `(len << 1) | tagBit` into a ULEB128:

  * `is_bytes == 1` → payload is a **lower-cased hex-encoded string**, represented as **raw bytes**.
  * `is_bytes == 0` → payload is **UTF‑8 text**.

> **Note:** `content` uses a plain `varint(len)` + bytes and is **always UTF‑8 text**; only tag elements are tagged as text/bytes.

### 3.3 String form

To produce a shareable string:

```
notepack-string = "notepack_" + base64_nopad(notepack-binary)
```

* Base64 alphabet per RFC 4648 standard **without "=" padding**.

---

## 4. Encoding rules (normative)

1. **Fixed-width fields**

   * `id`, `pubkey`, `sig` MUST be exactly 32, 32, and 64 bytes respectively in the binary form.
     (When converting from JSON‑like sources that use hex strings, the hex MUST decode to those lengths.)

2. **Varints**

   * `created_at`, `kind`, `content_len`, `num_tags`, and per‑tag `num_elems` MUST be encoded as ULEB128 of a non‑negative `u64`.
   * Decoders MUST reject varints that set bits beyond 64 total value bits (overflow) or run past the end of input (unterminated).

3. **Content**

   * `content` MUST be well‑formed UTF‑8. Empty content is encoded as `varint(0)` followed by no bytes.

4. **Tag elements (critical rule)**

   * For every element in a tag, first write a **tagged‑varint** with:
     `raw = (len << 1) | is_bytes`, then write `len` bytes of payload.
   * Encoders SHOULD choose **Bytes** for data that is truly binary (e.g., 32‑byte ids) and **Str** for human text.
     The reference encoder uses an aggressive heuristic: *if a tag element **string** is valid **lower-cased** hex, it is encoded as **Bytes***

5. **String wrapper**

   * The shareable string MUST start with the ASCII literal `notepack_` followed by Base64 (no padding) of the binary payload.

---

## 5. Decoding rules (normative)

* Parse in the order defined in §3.1.
* For `content`, read exactly `content_len` bytes and validate UTF‑8.
* For each tag element, read `tagged_varint` to determine `len` and `is_bytes`, then read exactly `len` bytes.
* Decoders MUST handle:

  * **Truncated** inputs (any read past end).
  * **VarintOverflow** (more than 64 bits of value).
  * **VarintUnterminated** (stream ended mid‑varint).
  * **UTF‑8 errors** in text fields.
  * **Base64 errors** for the string form.

> **Implementation tip:** Treat fixed-size reads (32/32/64) as failing with “truncated” if insufficient bytes remain.

---

## 6. Wire compatibility & round‑tripping

* The binary format is **lossless** for all fields except a subtlety in **tag elements**:

  * If an element **looks like hex**, the reference encoder emits it as **Bytes**, losing the knowledge that it was originally human text.
  * A decoder that wishes to re‑materialize a JSON‑like Note SHOULD hex‑encode **Bytes** elements in **lowercase** (the reference tools do so), but cannot recover original letter‑case or intent.

---

## 7. Error conditions

A conforming decoder MUST surface the following (names shown for clarity):

* **Truncated** — stream ended before a demanded length.
* **VarintOverflow** — more than 64 value bits accumulated.
* **VarintUnterminated** — ran out of bytes while varint still had continuation bit set.
* **Utf8** — `content` or a text tag element isn’t valid UTF‑8.
* **Base64Decode** — bad Base64 in the string form.

Encoders SHOULD validate that `id`, `pubkey`, and `sig` source material decode to the correct sizes.

---

## 8. Rationale & design notes

* **ULEB128 everywhere** keeps small numbers cheap (e.g., short content, few tags).
* **Tagged‑varint for tag elems** avoids separate type bytes and neatly piggybacks on the length prefix.
* **Version field** used to identify format changes when not using the base64 encoding
* **No signature/id recomputation**: notepack treats fields as data; it does not verify `id == sha256(serialized_event)`. Keep verification at a higher layer.

---

## 9. Test vectors

### 9.1 Minimal illustrative note

```
id      = 32 bytes of 0x00
pubkey  = 32 bytes of 0x11
sig     = 64 bytes of 0x22
created_at = 1720000000
kind       = 0
content    = "hello"
tags       = [
  ["e", <32 bytes of 0xaa>, "wss://relay.example.com"],
  ["p", <32 bytes of 0xbb>]
]
```

**Packed binary (hex):**

```
000000000000000000000000000000000000000000000000000000000000000011111111111111111111111111111111111111111111111111111111111111112222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222280bc94b406000568656c6c6f0203026541aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa2e7773733a2f2f72656c61792e6578616d706c652e636f6d02027041bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
```

Breakdown (annotated):

* `00…00` (32B) id
* `11…11` (32B) pubkey
* `22…22` (64B) sig
* `80 bc 94 b4 06` → varint(1720000000)
* `00` → varint(kind=0)
* `05` → varint(content\_len=5)
* `68 65 6c 6c 6f` → “hello”
* `02` → varint(num\_tags=2)

Tag #1:

* `03` → varint(num\_elems=3)
* `02 65` → tagged(len=1,is\_bytes=0) + "e"
* `41` → tagged(len=32,is\_bytes=1)
* `aa…aa` (32B)
* `2e` → tagged(len=23,is\_bytes=0)
* `77 73 73 3a 2f 2f 72 65 6c 61 79 2e 65 78 61 6d 70 6c 65 2e 63 6f 6d` → "wss\://relay.example.com"

Tag #2:

* `02` → varint(num\_elems=2)
* `02 70` → tagged(len=1,is\_bytes=0) + "p"
* `41` → tagged(len=32,is\_bytes=1)
* `bb…bb` (32B)

**String form (`notepack_` + Base64 without padding):**

```
notepack_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAARERERERERERERERERERERERERERERERERERERERERESIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiKAvJS0BgAFaGVsbG8CAwJlQaqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqLndzczovL3JlbGF5LmV4YW1wbGUuY29tAgJwQbu7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7
```

---

## 11. Pseudocode

### 11.1 Varint (ULEB128)

```text
write_varint(u64 n):
  do:
    b = n & 0x7f
    n >>= 7
    if n != 0: b |= 0x80
    emit b
  while n != 0

read_varint():
  shift = 0; n = 0
  loop over input bytes:
    b = next()
    n |= (b & 0x7f) << shift
    if (b & 0x80) == 0: return n
    shift += 7
    if shift >= 64: error(VarintOverflow)
  error(VarintUnterminated)
```

### 11.2 Tagged‑varint for tag elements

```text
write_tagged(len: u64, is_bytes: bool):
  raw = (len << 1) | (is_bytes ? 1 : 0)
  write_varint(raw)

read_tagged():
  raw = read_varint()
  len = raw >> 1
  is_bytes = (raw & 1) == 1
  return (len, is_bytes)
```

---

## 12. Conformance checklist

* [ ] Validate fixed sizes (32/32/64) for `id`, `pubkey`, `sig`.
* [ ] ULEB128 for all varints; enforce overflow/unterminated errors.
* [ ] `content` is UTF‑8 (reject invalid).
* [ ] Tag elements use **tagged‑varint**; honor `is_bytes` when decoding.
* [ ] String form uses `notepack_` + Base64 **without padding**.
* [ ] Enforce reasonable size limits (see §9).

---

## 13. Extensibility

* **Reserved:** No version field is present. If the binary layout changes incompatibly, introduce a new string prefix (e.g., `notepack1_`), or define envelope tags at the end guarded by a feature bit in `kind`.
* **Forward‑compat:** Decoders MUST stop exactly at the end of the payload; there is no trailing‑field discovery mechanism in 0.1.

---

## 14. Interop notes

* When down‑converting Bytes tag elements to textual formats, hex‑encode in **lowercase** to match common practice.
* If you need to disambiguate “hex text” from “bytes containing the same value,” change your producer to include a non‑hex character (e.g., `0x...`) so it is encoded as a **Str**.

[nip01]: https://github.com/nostr-protocol/nips/blob/master/01.md#events-and-signatures
