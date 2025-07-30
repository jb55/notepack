use notepack::{Note, pack_note_to_string};
use std::io;

fn main() {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).expect("line");
    let trimmed = buffer.trim();

    if trimmed.starts_with("notepack_") {
        // unpack
        let hex = notepack::unpack_note_from_string(&trimmed[9..]).expect("unpack ok");
        println!("{hex}");
    } else {
        // pack
        let note: Note = serde_json::from_str(trimmed).expect("decode ok");
        let packed = pack_note_to_string(&note).expect("packed ok");
        println!("{packed}");
    }
}
