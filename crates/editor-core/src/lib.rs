//! The editing core of `editor`: buffers, the cursor and modes, and the two keymaps. The
//! window, the views and the background services live in the binary (`src/main.rs`).

pub mod buffer;
pub mod classic;
pub mod editor;
pub mod find;
pub mod syntax;
pub mod vim;
