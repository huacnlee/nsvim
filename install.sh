#!/bin/sh
# Install the published nsvim release for the current user. No sudo is necessary.
set -eu

version="${NSVIM_VERSION:-latest}"
destination="${NSVIM_INSTALL_DIR:-}"
usage() {
  printf '%s\n' 'Usage: install.sh [--version VERSION] [--dir DIRECTORY]' \
    'Defaults: the latest release, ~/Applications on macOS, ~/.local on Linux.'
}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --version|--dir)
      [ "$#" -ge 2 ] || { usage >&2; exit 2; }
      if [ "$1" = --version ]; then version="$2"; else destination="$2"; fi
      shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done
die() { printf 'nsvim: %s\n' "$*" >&2; exit 1; }
for tool in curl tar awk; do command -v "$tool" >/dev/null || die "Required command missing: $tool"; done
[ -n "${HOME:-}" ] || die 'HOME is not set.'
case "$(uname -s)" in
  Darwin) platform=macos; suffix=apple-darwin; destination="${destination:-$HOME/Applications}"; bin_dir="$HOME/.local/bin" ;;
  Linux) platform=linux; suffix=unknown-linux-gnu; destination="${destination:-$HOME/.local}"; bin_dir="$destination/bin" ;;
  *) die 'Use install.ps1 on Windows. This script supports macOS and Linux.' ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) die 'Unsupported CPU architecture (requires x86_64 or arm64).' ;;
esac
# The command nsvim is a symbolic link. Do not replace a file that another program put there.
link="$bin_dir/nsvim"
if [ -e "$link" ] && [ ! -L "$link" ]; then
  die "$link is a file, not a link. Remove it, then run the installer again."
fi
download() { curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location "$@"; }
repo=https://github.com/huacnlee/nsvim
if [ "$version" = latest ]; then
  release_url="$(download --output /dev/null --write-out '%{url_effective}' "$repo/releases/latest")"
  version="${release_url##*/}"
fi
version="${version#v}"
awk -v version="$version" 'BEGIN {exit !(version ~ /^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$/)}' || die 'Version must be a release version such as 0.4.0 or v0.4.0.'
asset="nsvim-$version-$arch-$suffix.tar.gz"
base="$repo/releases/download/v$version"
work="$(mktemp -d "${TMPDIR:-/tmp}/nsvim-install.XXXXXXXX")"
trap 'rm -rf "$work"' 0
printf 'Downloading nsvim %s for %s %s...\n' "$version" "$platform" "$arch"
download "$base/$asset" --output "$work/$asset" || die "Release v$version has no $asset. Use a later release."
download "$base/SHA256SUMS" --output "$work/SHA256SUMS" || die "Release v$version has no SHA256SUMS."
expected="$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "$work/SHA256SUMS")"
[ "${#expected}" -eq 64 ] || die "Missing or invalid checksum for $asset"
case "$expected" in *[!a-fA-F0-9]*) die "Missing or invalid checksum for $asset" ;; esac
if command -v sha256sum >/dev/null; then
  actual="$(sha256sum "$work/$asset")"
elif command -v shasum >/dev/null; then
  actual="$(shasum -a 256 "$work/$asset")"
else die 'Required command missing: sha256sum or shasum'; fi
[ "${actual%% *}" = "$expected" ] || die 'Checksum verification failed. The installer installed nothing.'
mkdir "$work/unpacked"
# Accept only plain files and folders, with relative paths that stay in the extraction folder.
tar -tzf "$work/$asset" > "$work/entries"
tar -tvzf "$work/$asset" > "$work/listing"
if awk '/^\// || /(^|\/)\.\.(\/|$)/ {bad=1} END {exit !bad}' "$work/entries" ||
  awk '!/^[-d]/ {bad=1} END {exit !bad}' "$work/listing"; then
  die 'Release archive contains an unsafe path.'
fi
tar -xzf "$work/$asset" -C "$work/unpacked"

# replace PARENT NAME SOURCE: put the folder SOURCE at PARENT/NAME as one unit. When the move
# fails, the previous version comes back.
replace() {
  staging="$(mktemp -d "$1/.nsvim-install.XXXXXXXX")"
  trap 'rm -rf "$work" "${staging:-}"' 0
  if [ "$platform" = macos ]; then ditto "$3" "$staging/$2"; else cp -R "$3" "$staging/$2"; fi
  previous="$staging/previous"
  if [ -e "$1/$2" ]; then mv "$1/$2" "$previous"; fi
  if ! mv "$staging/$2" "$1/$2"; then
    if [ -e "$previous" ]; then mv "$previous" "$1/$2"; fi
    die "Could not replace $1/$2."
  fi
  rm -rf "$staging"
}

if [ "$platform" = macos ]; then
  app="$work/unpacked/nsvim.app"
  for file in Contents/Info.plist Contents/MacOS/nsvim Contents/Resources/nsh/bundle.nshc; do
    [ -f "$app/$file" ] || die "Release archive is missing nsvim.app/$file."
  done
  mkdir -p "$destination"
  replace "$destination" nsvim.app "$app"
  program="$destination/nsvim.app/Contents/MacOS/nsvim"
else
  source_dir="$work/unpacked"
  # The program reads its interface from nsh/ next to the real executable.
  for file in nsvim nsh/bundle.nshc packaging/nsvim.desktop packaging/nsvim.svg; do
    [ -f "$source_dir/$file" ] || die "Release archive is missing $file."
  done
  mkdir "$work/program"
  for file in nsvim nsh LICENSE README.md; do
    if [ -e "$source_dir/$file" ]; then cp -R "$source_dir/$file" "$work/program/$file"; fi
  done
  chmod 755 "$work/program/nsvim"
  mkdir -p "$destination/share/applications" "$destination/share/icons/hicolor/scalable/apps"
  replace "$destination/share" nsvim "$work/program"
  program="$destination/share/nsvim/nsvim"
  # Launchers do not always have the bin folder on PATH, so the entry names the program in full.
  escaped="$(printf '%s' "$program" | sed -e 's/[\\"`$]/\\\\&/g' -e 's/%/%%/g')"
  NSVIM_EXEC="Exec=\"$escaped\" %F" awk '/^Exec=/ { print ENVIRON["NSVIM_EXEC"]; next } { print }' \
    "$source_dir/packaging/nsvim.desktop" > "$work/nsvim.desktop"
  install -m644 "$work/nsvim.desktop" "$destination/share/applications/nsvim.desktop"
  install -m644 "$source_dir/packaging/nsvim.svg" "$destination/share/icons/hicolor/scalable/apps/nsvim.svg"
  if command -v update-desktop-database >/dev/null; then update-desktop-database "$destination/share/applications" >/dev/null 2>&1 || true; fi
fi
mkdir -p "$bin_dir"
ln -sfn "$program" "$link"
if [ "$platform" = macos ]; then
  printf 'Installed %s/nsvim.app\n' "$destination"
else
  printf 'Installed %s\n' "$destination/share/nsvim"
fi
printf 'Linked %s\n' "$link"
case ":${PATH:-}:" in
  *":$bin_dir:"*) printf '%s\n' 'Run nsvim . in a project folder. The first launch runs a short setup.' ;;
  *) printf 'Add %s to PATH, then run nsvim . in a project folder.\n' "$bin_dir" ;;
esac
if [ "$platform" = linux ]; then
  printf '%s\n' 'nsvim needs GTK 4.14 or later.'
fi
