use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use notepack::{NoteBuf, NoteParser, StringType, pack_note, pack_note_to_string};
use std::hint::black_box;

const CONTACTS_JSON: &str = include_str!("contact-list.json");

fn parse_iter_only(bytes: &[u8]) -> usize {
    let parser = NoteParser::new(bytes);
    let mut n = 0usize;
    for f in parser {
        let _ = f.expect("parse ok");
        n += 1;
    }
    n
}

fn bench_decode(c: &mut Criterion) {
    // One-time fixture setup outside the timer.
    let json_len = CONTACTS_JSON.as_bytes().len() as u64;

    let note_from_json: NoteBuf =
        serde_json::from_str(CONTACTS_JSON).expect("valid fixture");
    let notepack_bytes = pack_note(&note_from_json).expect("pack ok");
    let notepack_b64 = pack_note_to_string(&note_from_json).expect("pack to string ok");

    // 1) Streaming-only lower bound: iterate fields without allocating a Note.
    {
        let mut group = c.benchmark_group("contacts/notepack_stream_iter_only");
        group.throughput(Throughput::Bytes(notepack_bytes.len() as u64));
        group.bench_function("contacts.np.iter", |b| {
            b.iter(|| {
                let nfields = parse_iter_only(black_box(&notepack_bytes));
                black_box(nfields);
            });
        });
        group.finish();
    }

    // 2) Base64 notepack string -> bytes -> Note
    {
        let mut group = c.benchmark_group("contacts/notepack_decode_b64_and_parse");
        group.throughput(Throughput::Bytes(notepack_b64.len() as u64));
        group.bench_function("contacts.np.b64", |b| {
            b.iter(|| {
                // Intentionally measure both the decode and parse.
                let bs = NoteParser::decode(black_box(&notepack_b64)).expect("decode ok");
                let note = NoteParser::new(&bs).into_note().expect("parse ok");
                black_box(note);
            });
        });
        group.finish();
    }

    // 3) Notepack bytes -> Note
    {
        let mut group = c.benchmark_group("contacts/notepack_parse_bytes");
        group.throughput(Throughput::Bytes(notepack_bytes.len() as u64));
        group.bench_function("contacts.np", |b| {
            b.iter(|| {
                let note = NoteParser::new(black_box(&notepack_bytes))
                    .into_note()
                    .expect("parse ok");
                black_box(note);
            });
        });
        group.finish();
    }

    // 4) Notepack bytes -> Note + iterate all tags (forces reading all bytes)
    {
        let mut group = c.benchmark_group("contacts/notepack_parse_bytes_and_iter_tags");
        group.throughput(Throughput::Bytes(notepack_bytes.len() as u64));
        group.bench_function("contacts.np.tags", |b| {
            b.iter(|| {
                let note = NoteParser::new(black_box(&notepack_bytes))
                    .into_note()
                    .expect("parse ok");

                // Drain tags so we actually read everything claimed in throughput.
                let mut tags = note.tags; // move out; no need to clone
                let mut acc = 0usize;
                while let Some(mut elems) = tags.next_tag().expect("tag ok") {
                    for item in &mut elems {
                        match item.expect("elem ok") {
                            StringType::Str(s) => acc += s.len(),
                            StringType::Bytes(bs) => acc += bs.len(),
                        }
                    }
                }
                black_box(acc)
            });
        });
        group.finish();
    }

    // 5) JSON -> NoteBuf
    {
        let mut group = c.benchmark_group("contacts/json_from_str");
        group.throughput(Throughput::Bytes(json_len));
        group.bench_function("contacts.json", |b| {
            b.iter(|| {
                let note: NoteBuf =
                    serde_json::from_str(black_box(CONTACTS_JSON)).expect("json->note");
                black_box(note);
            });
        });
        group.finish();
    }

    // 5) JSON -> NoteBuf + iterate tags
    {
        let mut group = c.benchmark_group("contacts/json_from_str_iter");
        group.throughput(Throughput::Bytes(json_len));
        group.bench_function("contacts.json.iter", |b| {
            b.iter(|| {
                let note: NoteBuf =
                    serde_json::from_str(black_box(CONTACTS_JSON)).expect("json->note");

                // apples-to-apples iterate comparison
                let mut tags = note.tags; // move out; no need to clone
                let mut acc = 0usize;
                for tag in tags {
                    for elem in tag {
                        acc += elem.len();
                    }
                }
                black_box(acc)
            });
        });
        group.finish();
    }
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
