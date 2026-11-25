Closes #[issue number]

## Description

Say in English what this pull request changes and which problem it solves.
Keep **one pull request for one problem**, and keep **small changes small**. Then it is easy to review and to merge.

## Screenshot

| Platform | Before                       | After                       |
| -------- | ---------------------------- | --------------------------- |
| Linux    | [Put Before Screenshot here] | [Put After Screenshot here] |
| macOS    | [Put Before Screenshot here] | [Put After Screenshot here] |
| Windows  | [Put Before Screenshot here] | [Put After Screenshot here] |

## User-facing changes

List each thing a user or a script sees that this pull request adds, changes or removes: keys, `:` commands,
`config.toml` keys, command line options, `ctl` and `mcp` requests, installer options and release files. Give one
line on what each is for. If none, remove this section.

- `config.toml`: `editor.example = true`: what it does.

## Breaking Changes

Describe each change that breaks a released behavior: a key, a command, a config key or the release layout. If
none, remove this section.

```diff
- Old usage
+ New usage
```

## Performance

For changes to a hot path (a key press, a scroll, the first frame): the before and after numbers of the bench
tests in [docs/performance.md](../docs/performance.md). If none, remove this section.

## How to Test

Describe the tests that you ran to verify your changes. Give the steps so that others can do them again.

## Checklist

- [ ] I have read the [CONTRIBUTING](../CONTRIBUTING.md) document and followed the guidelines.
- [ ] Reviewed the changes in this PR and confirmed AI generated code (if any) is accurate.
- [ ] Passed `cargo test`, `cargo fmt` and `cargo clippy --all-targets -- -D warnings`.
- [ ] Passed `nsh check` and `nsh build` (if the change touches `App.tsx` or `src/`).
- [ ] Ran the app on the affected platforms with a scratch `HOME` (if the change is visible or platform-specific).
