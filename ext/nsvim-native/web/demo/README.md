# notes

A small notes tool. This project is a demo: it lives in the page's memory, and nothing you
change here goes to a disk.

## Try these keys

The demo uses the classic keymap: there are no modes, and what you type goes in the text.
In the browser the keys take `Ctrl`, on macOS too.

- Click in the text and type. Arrows move, `⇧` and an arrow select.
- `Ctrl C`, `Ctrl X` and `Ctrl V` copy, cut and paste.
- `Ctrl Z` undoes and `Ctrl ⇧ Z` redoes.
- `Ctrl F` finds in the file. `Ctrl ⇧ F` searches the project.
- `Ctrl P` goes to a file. `Ctrl B` shows or hides the file tree.
- `Ctrl S` saves the file to memory. `F1` lists every key.

NsVim also has a full Vim keymap. On your computer, pick it at the first launch.

## Files

- `src/main.rs` reads the notes and prints them.
- `src/note.rs` holds one note and its tags.
- `Cargo.toml` names the crate.
