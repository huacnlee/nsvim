# nsvim app

This crate is the nsvim app. It draws the editor with [NSH](https://nsh.dev), on the native
views of each platform: GTK 4 on Linux, AppKit on macOS and WinUI on Windows. The editing core
is the crate `editor-core` in `../crates/editor-core`.

Build, test and run it from the repository root. See [docs/development.md](../docs/development.md)
and [docs/architecture.md](../docs/architecture.md).

## Known limits of NSH that the app works around

- NSH 0.7.0 or later. On 0.6.1, a Rust root with a `window()` opens two windows on Linux.
- On Linux, `focus()` works only after the window shows. The app asks again until the key view
  has the focus. A press in the app gives the focus back to the key view.
- The root view of a window gets no `on_layout`. Layout listeners sit on inner views.
- On GTK, the border of a view paints over its children. A box with a title in its top edge
  draws its border in a child under the title (`ui::style::frame`).
- In Headless tests, a `window()` root shows only on a desktop OS. The tests set `Os::Linux`.
