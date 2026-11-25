#!/bin/sh
# Builds nsvim for the web and puts the static site in website/public/demo/.
#
# It needs Rust with the wasm32-unknown-unknown target, clang, llvm-ar and the NSH CLI 0.7.0.
# Run it from any folder. Give another output folder as the first argument.
set -eu

repo=$(cd "$(dirname "$0")/../.." && pwd)
dest=${1:-"$repo/website/public/demo"}
work="$repo/nsh/target/web"

# The tree-sitter parsers are C. A browser gives C no library, so the C code compiles against
# the small headers in nsh/web/libc. The functions behind them are in nsh/src/web_libc.rs.
export CFLAGS_wasm32_unknown_unknown="-isystem $repo/nsh/web/libc/include"

cd "$repo"
rm -rf "$work"
nsh package --target web --out "$work"

# The site's server compresses files itself. The .br and .gz copies only make the site larger.
rm -rf "$dest"
mkdir -p "$dest"
cp -R "$work/nsvim/." "$dest"
find "$dest" -type f \( -name '*.br' -o -name '*.gz' \) -delete
echo "nsvim for the web: $dest ($(du -sh "$dest" | cut -f1))"
