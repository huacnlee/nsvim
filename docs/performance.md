# Performance

Speed decides every feature. These are the targets from the design, and what a release build
measures on Linux, x86-64.

| Target | Result | How |
| --- | --- | --- |
| Open a 100 MB file in under 1 s | 175 ms | `big_file_opens_and_edits_fast` |
| An edit, undo and jump in a 100 MB file | 24 µs | the same test |
| Keystroke to frame in under 8 ms | 0.3 ms median, 1.4 ms at the 95th percentile | 60 keys, headless, 2× |
| Typing in a 200,000-line file | 0.5 ms a key, frame included | `bench_big_file_keys` |
| The first edit after launch | 0.8 ms | `bench_first_edit_small_file` |
| Finder ranking in a 200,000-file project | 1.1 to 2.0 ms a key | `bench_finder_and_tree` |
| The file tree of a 200,000-file project | 22 ms, on the index worker | `bench_finder_and_tree` |
| `:%s/self/this/g` over 200,000 lines | 182 ms | `bench_substitute_and_search` |
| Launch to the first frame | 105 to 135 ms, most of it window and GPU setup | `EDITOR_TIMING=1` |
| An idle window draws nothing | One frame per key, then none | headless frame counts |
| No LSP or file I/O on the UI thread | Workers do all of it | [architecture](architecture.md) |

## What keeps it fast

- **A rope.** [ropey](https://docs.rs/ropey) makes line lookups, inserts and removes cheap at
  any file size.
- **Only what shows.** The view builds the visible lines, tree rows and picker rows.
- **Syntax colours by window.** The lexers run over the lines on screen plus 300 above them,
  aligned to 64-line blocks and cached per buffer version.
- **Instant changes.** Text and the cursor change in the next frame with no animation, so a key
  costs one frame.
- **Workers.** Opening, saving, indexing, git, ripgrep, file watching and LSP run on their own
  threads.
- **A cheap finder.** Ranking scores ASCII paths as bytes, with no allocation per path. It keeps
  only the best 200 matches and finds hit positions for those alone. A list over 16,384 paths
  splits across up to 8 threads.
- **The tree off the UI thread.** The index worker walks the project on several threads and
  builds the file tree. The UI thread only swaps the result in. The git worker lists the folders
  with changes, so a folder's mark is one lookup.
- **Fonts ready before the first key.** The first symbol outside the UI font, such as the unsaved
  dot, makes Rune load a fallback font in about 12 ms. The second frame draws those symbols
  unseen, so no key pays for it. Setup loads the system font list while the workers open files.
- **A search that skips.** When every branch of a pattern starts with a plain char, the regex
  tries a match only where the line has that char. ASCII case folds without the Unicode tables.
- **Link-time optimization.** Release builds use fat LTO, one codegen unit and stripped symbols:
  45.7 MB instead of 63.6 MB. `cargo build --profile quick` builds the old way, for quick local
  checks.
- **Limits.** Buffers over 4 MiB skip git signs and LSP. Swap files stop at 16 MiB. Tree-sitter
  skips files over 4 MiB. The index stops at 200,000 files.

## Measure it yourself

Run the benchmarks. Each prints its timings, the best of several runs where it repeats:

```sh
cargo test --release --bin editor bench_ -- --ignored --nocapture --test-threads 1
```

The release profile links with LTO, so the first build takes about 5 minutes. Use
`--profile quick` in place of `--release` while you iterate. `BENCH_WALK=/path/to/project`
times the index walk on that folder.

See where launch time goes. Each line is milliseconds since `main` started:

```sh
EDITOR_TIMING=1 nsvim --new src/main.rs
```

```text
timing:     5.10 ms  window ready, starting workers
timing:     5.60 ms  workers started
timing:    30.50 ms  font list loaded
timing:    87.40 ms  poll 1 (frame 1 is next)
timing:   131.80 ms  poll 2 (frame 2 is next)
```

Open and edit a large file:

```sh
python3 -c "
import sys
with open('/tmp/huge.rs', 'w') as f:
    for i in range(1_050_000):
        f.write(f'    let value_{i} = compute(\"item {i}\", {i}) + offset; // eighty chars\n')
"
EDITOR_BIG=/tmp/huge.rs cargo test --release --lib big_file -- --ignored --nocapture
```

Time keystrokes headless. A script types keys, and Rune prints the CPU time per frame:

```sh
cargo build --release
target/release/editor --scene editor --script keys.json --snap out.png --at 5 --scale 2
```

```json
{"events": [
  {"t": 0.3, "type": "type", "text": "j"},
  {"t": 0.35, "type": "type", "text": "j"},
  {"t": 0.4, "type": "key", "name": "d", "ctrl": true}
]}
```

The output has lines such as:

```text
rune: 301 frames at 60 fps, frame cpu mean 0.578 ms p50 0.289 p95 1.440
rune: animating frames 27, at-rest frames 273 (the window draws none)
```

Headless numbers leave out GPU upload and presentation. The 27 animating frames come from
startup, when layout settles.
