# notes

A small notes tool. This project is a demo: it lives in the page's memory, and nothing you
change here goes to a disk.

## Try these keys

- `j` and `k` move down and up. `w` and `b` move by words.
- `i` starts Insert mode. `Esc` goes back to Normal mode.
- `v` starts Visual mode. `y` copies, `d` cuts, `p` pastes.
- `/` searches the file. `n` goes to the next match.
- `Space f` finds a file. `Space /` searches the project.
- `Space e` shows the file tree. `:w` writes the file to memory.
- `u` undoes and `Ctrl r` redoes.

## Files

- `src/main.rs` reads the notes and prints them.
- `src/note.rs` holds one note and its tags.
- `Cargo.toml` names the crate.
