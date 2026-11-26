mod note;

use note::Note;

/// The notes to show when the file has none.
const WELCOME: &str = "Welcome to NsVim #start";

fn main() {
    let mut notes = vec![Note::parse(WELCOME)];
    notes.push(Note::parse("Read the README #docs"));
    notes.push(Note::parse("Try Visual mode with v #keys"));

    for (i, note) in notes.iter().enumerate() {
        println!("{:>2}. {}", i + 1, note);
    }

    let tagged = notes.iter().filter(|n| n.has_tag("keys")).count();
    println!("{tagged} note(s) about keys");
}
