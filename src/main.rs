use notepack::{Error, Note, NoteParser, ParsedField, StringType, pack_note_to_string};
use std::io;

fn main() -> Result<(), Error> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).expect("line");
    let trimmed = buffer.trim();

    if let Ok(packed) = NoteParser::decode(buffer.trim()) {
        let parser = NoteParser::new(&packed);
        let mut note = Note::default();
        for field in parser {
            process_field(&mut note, field?);
        }
        println!("{}", serde_json::to_string(&note)?);
    } else {
        let note: Note = serde_json::from_str(trimmed).expect("decode ok");
        let packed = pack_note_to_string(&note).expect("packed ok");
        println!("{packed}");
    }

    Ok(())
}

fn process_field(note: &mut Note, field: ParsedField<'_>) {
    match field {
        ParsedField::Version(_v) => {}
        ParsedField::Id(id) => {
            note.id = hex::encode(id);
        }
        ParsedField::Pubkey(pk) => {
            note.pubkey = hex::encode(pk);
        }
        ParsedField::Sig(sig) => {
            note.sig = hex::encode(sig);
        }
        ParsedField::CreatedAt(ts) => {
            note.created_at = ts;
        }
        ParsedField::Kind(kind) => {
            note.kind = kind;
        }
        ParsedField::Content(content) => {
            note.content = content.to_string();
        }
        ParsedField::NumTags(n) => note.tags = Vec::with_capacity(n as usize),
        ParsedField::NumTagElems(n) => note.tags.push(Vec::with_capacity(n as usize)),
        ParsedField::Tag(tag) => {
            let ind = note.tags.len() - 1;
            let current = &mut note.tags[ind];
            match tag {
                StringType::Bytes(bs) => {
                    current.push(hex::encode(bs));
                }
                StringType::Str(s) => {
                    current.push(s.to_string());
                }
            }
        }
    }
}

/*
fn print_field(field: ParsedField<'_>) {
    match field {
        ParsedField::Version(v) => eprintln!("version: {}", v),
        ParsedField::Id(id) => eprintln!("id: {}", hex::encode(id)),
        ParsedField::Pubkey(pk) => eprintln!("pk: {}", hex::encode(pk)),
        ParsedField::Sig(sig) => eprintln!("sig: {}", hex::encode(sig)),
        ParsedField::CreatedAt(ts) => eprintln!("created_at: {}", ts),
        ParsedField::Kind(kind) => eprintln!("kind: {}", kind),
        ParsedField::Content(content) => eprintln!("content: '{}'", content),
        ParsedField::NumTags(_n) => {}
        ParsedField::NumTagElems(_n) => {
            eprintln!()
        }
        ParsedField::Tag(tag) => match tag {
            StringType::Bytes(bs) => eprint!(" b:{}", hex::encode(bs)),
            StringType::Str(s) => eprint!(" s:{}", s),
        },
    }
}
*/
