# Performance

Speed decides every feature. The table shows what the bench tests measure on Linux, x86-64,
in an optimized build (`--profile quick`). The app tests run the real window in NSH's Headless
engine, so a key time includes the update, the memos and the layout, but not the GPU.

| Target | Result | How |
| --- | --- | --- |
| An edit in the core | 0.009 ms for the first, then under 0.002 ms | `bench_first_edit_in_core` |
| A key in a 200,000-line file | 0.15 to 0.6 ms, 2.7 ms at worst | `bench_big_file_keys` |
| Typing a line in a 200,000-line file | 0.8 ms for the worst key | `bench_big_file_keys` |
| Search in a 200,000-line file | 0.8 ms for `/needle`, 0.5 ms for `n` | `bench_big_file_keys` |
| The text area's frame at line 1000 | 0.21 ms for 40 rows, colours included | `bench_highlight_window` |
| Finder ranking in a 200,000-file project | 1.1 to 2.2 ms a key | `bench_finder_and_tree` |
| The file tree of a 200,000-file project | 24 ms, on the index worker | `bench_finder_and_tree` |
| `:%s/self/this/g` over 200,000 lines | 182 ms | `bench_substitute_and_search` |
| Engine start to the file on screen | 21 ms | `bench_first_frame` |
| No LSP or file I/O on the engine thread | Workers do all of it | [architecture](architecture.md) |

The first edit after launch costs 0.25 to 0.65 ms of engine-thread time, and the next keys 0.4
to 0.7 ms (`bench_first_edit_small_file`). That bench also fails if the engine thread reads or
writes a file, or starts a process, while it handles keys. In Headless tests the first key
reports about 130 ms of wall time. That is the test driver, which waits for the language server
and file watcher events of the open. It is not work, and a real window does not wait for it.

## What keeps it fast

- **A rope.** [ropey](https://docs.rs/ropey) makes line lookups, inserts and removes cheap at
  any file size.
- **Only what changed.** Each view reads its part of the app through a memo. A key runs again
  only the views whose part changed. The text area has one slot for each visible row, and a
  slot builds again only when its row changed.
- **Only what shows.** The frame holds the visible lines. The tree and the pickers show only the
  rows on screen.
- **Syntax colours by window.** The tree-sitter highlight query runs over the lines on screen.
- **Workers.** Opening, saving, indexing, git, ripgrep, file watching, LSP and the setup's
  checks run on their own threads.
- **A cheap finder.** Ranking scores ASCII paths as bytes, with no allocation per path. It keeps
  only the best 200 matches and finds hit positions for those alone. A list over 16,384 paths
  splits across up to 8 threads.
- **A search that skips.** When every branch of a pattern starts with a plain char, the regex
  tries a match only where the line has that char. ASCII case folds without the Unicode tables.
- **Link-time optimization.** Release builds use fat LTO, one codegen unit and stripped
  symbols. `cargo build --profile quick` builds without LTO, for quick local checks.
- **Limits.** Buffers over 4 MiB skip git signs and LSP. Swap files stop at 16 MiB. Tree-sitter
  skips files over 4 MiB. The index stops at 200,000 files, the first ones in name order, so a
  bigger folder shows the same part of itself after each change.

## Measure it yourself

Run the benchmarks. Each prints its timings and checks a time budget:

```sh
TMPDIR=~/.cache/nsvim-test-tmp cargo test --profile quick -p nsvim-native \
  --test flows_bench -- --ignored --nocapture --test-threads=1
```

See where launch time goes. Each line is milliseconds since `main` started:

```sh
EDITOR_TIMING=1 nvim --new src/main.rs
```

Open and edit a large file in the core. Make the file in a folder outside `/tmp`:

```sh
python3 -c "
with open('huge.rs', 'w') as f:
    for i in range(1_050_000):
        f.write(f'    let value_{i} = compute(\"item {i}\", {i}) + offset; // eighty chars\n')
"
EDITOR_BIG=$PWD/huge.rs cargo test --release -p editor-core big_file -- --ignored --nocapture
```
