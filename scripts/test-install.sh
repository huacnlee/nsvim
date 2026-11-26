#!/bin/sh
# Test install.sh with local release fixtures. The test mocks curl and uname, and it keeps HOME
# and every install in a scratch folder, so it never changes the real system.
# Usage: scripts/test-install.sh [SHELL]   (the shell that runs install.sh, default sh)
# check evaluates its condition later, so the conditions are in single quotes.
# shellcheck disable=SC2016,SC2034
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
script="$here/install.sh"
shell="${1:-sh}"
root="$(mktemp -d "${TMPDIR:-/tmp}/nsvim-install-test.XXXXXXXX")"
trap 'rm -rf "$root"' 0
failures=0
version=0.4.0

mkdir -p "$root/mock" "$root/fixtures"
cat > "$root/mock/uname" <<'EOF'
#!/bin/sh
if [ "$1" = -s ]; then echo "${MOCK_OS:-Linux}"; else echo "${MOCK_ARCH:-x86_64}"; fi
EOF
# The mock curl serves files from $FIXTURES by the last part of the URL.
cat > "$root/mock/curl" <<'EOF'
#!/bin/sh
url= output=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) output="$2"; shift ;;
    https://*) url="$1" ;;
  esac
  shift
done
case "$url" in
  https://github.com/huacnlee/nsvim/releases/latest)
    printf '%s' "https://github.com/huacnlee/nsvim/releases/tag/v$MOCK_LATEST"; exit 0 ;;
  https://github.com/huacnlee/nsvim/releases/download/*) ;;
  *) echo "unexpected URL: $url" >&2; exit 2 ;;
esac
file="$FIXTURES/${url##*/}"
[ -f "$file" ] || { echo "curl: (22) The requested URL returned error: 404" >&2; exit 22; }
cp "$file" "$output"
EOF
# macOS copies the app with ditto. A copy is enough for the test.
cat > "$root/mock/ditto" <<'EOF'
#!/bin/sh
cp -R "$1" "$2"
EOF
chmod 755 "$root/mock/uname" "$root/mock/curl" "$root/mock/ditto"

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | awk '{print $1}'
}

# release TARGET [unsafe|link|no-bundle|old-app]: write the archive for TARGET and its SHA256SUMS.
# old-app: a macOS release from before the bundle was NsVim.app.
release() {
  rm -rf "$root/stage" && mkdir -p "$root/stage/pack"
  pack="$root/stage/pack"
  case "$1" in
    *apple-darwin)
      bundle=NsVim.app
      if [ "${2:-}" = old-app ]; then bundle=nsvim.app; fi
      contents="$pack/$bundle/Contents"
      mkdir -p "$contents/MacOS" "$contents/Resources/nsh"
      printf '#!/bin/sh\necho "nsvim %s"\n' "$version" > "$contents/MacOS/nsvim"
      chmod 755 "$contents/MacOS/nsvim"
      echo plist > "$contents/Info.plist"
      echo bytecode > "$contents/Resources/nsh/bundle.nshc" ;;
    *)
      mkdir -p "$pack/nsh" "$pack/packaging"
      printf '#!/bin/sh\necho "nsvim %s"\n' "$version" > "$pack/nsvim"
      chmod 755 "$pack/nsvim"
      echo bytecode > "$pack/nsh/bundle.nshc"
      echo '{"id":"dev.nsvim.app"}' > "$pack/nsh/app.json"
      printf '[Desktop Entry]\nName=nsvim\nExec=nsvim %%F\nIcon=nsvim\n' > "$pack/packaging/nsvim.desktop"
      echo '<svg/>' > "$pack/packaging/nsvim.svg"
      echo license > "$pack/LICENSE" ;;
  esac
  case "${2:-}" in
    no-bundle) rm -f "$pack/nsh/bundle.nshc" ;;
    link) ln -s /etc/passwd "$pack/escape" ;;
  esac
  archive="$root/fixtures/nsvim-$version-$1.tar.gz"
  if [ "${2:-}" = unsafe ]; then
    mkdir -p "$root/stage/pack/sub" && echo bad > "$root/stage/pack/escaped"
    (cd "$pack/sub" && tar -P -czf "$archive" ../escaped ../nsvim ../nsh ../packaging)
  else
    (cd "$pack" && tar -czf "$archive" .)
  fi
  printf '%s  %s\n' "$(sha256 "$archive")" "${archive##*/}" > "$root/fixtures/SHA256SUMS"
}

# run_install ARGS...: run install.sh with the mocks. Output goes to $root/out.
run_install() {
  status=0
  env PATH="$root/mock:$PATH" HOME="$root/home" FIXTURES="$root/fixtures" MOCK_LATEST="$version" \
    TMPDIR="$root/tmp" "$shell" "$script" "$@" > "$root/out" 2>&1 || status=$?
  return 0
}

fresh() {
  rm -rf "${root:?}/home" "$root/tmp" "$root/fixtures" "$root/dest"
  mkdir -p "$root/home" "$root/tmp" "$root/fixtures"
  dest="$root/dest/install with spaces"
  unset MOCK_OS MOCK_ARCH || true
}

check() {
  if eval "$2"; then
    :
  else
    printf 'FAIL %s: %s\n' "$1" "$2"
    sed 's/^/  | /' "$root/out" 2>/dev/null || true
    failures=$((failures + 1))
    return 1
  fi
}
ok() { printf 'PASS %s\n' "$1"; }

# A good Linux install, then an upgrade over it.
name='linux install and upgrade'
fresh
release x86_64-unknown-linux-gnu
run_install --dir "$dest"
check "$name" '[ "$status" -eq 0 ]' &&
  run_install --dir "$dest" &&
  check "$name" '[ "$status" -eq 0 ]' &&
  check "$name" '[ -x "$dest/share/nsvim/nsvim" ] && [ -f "$dest/share/nsvim/nsh/bundle.nshc" ]' &&
  check "$name" '[ -f "$dest/share/nsvim/nsh/app.json" ] && [ -f "$dest/share/nsvim/LICENSE" ]' &&
  check "$name" '[ ! -e "$dest/bin" ] && [ -x "$root/home/.local/bin/nsvim" ]' &&
  check "$name" '[ "$("$dest/share/nsvim/nsvim")" = "nsvim $version" ]' &&
  check "$name" '[ "$("$root/home/.local/bin/nsvim")" = "nsvim $version" ]' &&
  check "$name" 'grep -q "Add $root/home/.local/bin to PATH" "$root/out"' &&
  check "$name" 'grep -q "run: $dest/share/nsvim/nsvim \." "$root/out" && grep -q "make nvim, vim and vi open nsvim" "$root/out"' &&
  check "$name" 'grep -qxF "Exec=\"$dest/share/nsvim/nsvim\" %F" "$dest/share/applications/nsvim.desktop"' &&
  check "$name" '[ -f "$dest/share/icons/hicolor/scalable/apps/nsvim.svg" ]' &&
  check "$name" '[ -z "$(ls -A "$dest/share" | grep nsvim-install || true)" ]' &&
  check "$name" '[ -z "$(ls -A "$root/tmp")" ]' &&
  ok "$name"

name='linux arm64 install'
fresh
release aarch64-unknown-linux-gnu
MOCK_ARCH=aarch64; export MOCK_ARCH
run_install --dir "$dest" --version v$version
check "$name" '[ "$status" -eq 0 ] && [ -x "$dest/share/nsvim/nsvim" ]' && ok "$name"

name='pipe to sh with NSVIM_INSTALL_DIR'
fresh
release x86_64-unknown-linux-gnu
status=0
env PATH="$root/mock:$PATH" HOME="$root/home" FIXTURES="$root/fixtures" MOCK_LATEST="$version" \
  TMPDIR="$root/tmp" NSVIM_INSTALL_DIR="$dest" "$shell" < "$script" > "$root/out" 2>&1 || status=$?
check "$name" '[ "$status" -eq 0 ] && [ -x "$dest/share/nsvim/nsvim" ]' && ok "$name"

name='default prefix is ~/.local'
fresh
release x86_64-unknown-linux-gnu
run_install
check "$name" '[ "$status" -eq 0 ] && [ -x "$root/home/.local/share/nsvim/nsvim" ]' &&
  check "$name" '[ -f "$root/home/.local/share/applications/nsvim.desktop" ]' && ok "$name"

name='bad checksum installs nothing'
fresh
release x86_64-unknown-linux-gnu
echo tampered >> "$root/fixtures/nsvim-$version-x86_64-unknown-linux-gnu.tar.gz"
run_install --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "Checksum verification failed" "$root/out" && [ ! -e "$dest" ]' && ok "$name"

name='missing checksum installs nothing'
fresh
release x86_64-unknown-linux-gnu
echo "0000000000000000000000000000000000000000000000000000000000000000  other.tar.gz" > "$root/fixtures/SHA256SUMS"
run_install --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "Missing or invalid checksum" "$root/out" && [ ! -e "$dest" ]' && ok "$name"

name='unsafe path installs nothing'
fresh
release x86_64-unknown-linux-gnu unsafe
run_install --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "unsafe path" "$root/out" && [ ! -e "$dest" ] && [ ! -e "$root/dest/escaped" ]' && ok "$name"

name='symbolic link in the archive installs nothing'
fresh
release x86_64-unknown-linux-gnu link
run_install --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "unsafe path" "$root/out" && [ ! -e "$dest" ]' && ok "$name"

name='archive without the interface bundle installs nothing'
fresh
release x86_64-unknown-linux-gnu no-bundle
run_install --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "missing nsh/bundle.nshc" "$root/out" && [ ! -e "$dest" ]' && ok "$name"

name='release without installer assets'
fresh
release x86_64-unknown-linux-gnu
run_install --dir "$dest" --version 0.3.2
check "$name" '[ "$status" -ne 0 ] && grep -q "Release v0.3.2 has no nsvim-0.3.2-x86_64-unknown-linux-gnu.tar.gz" "$root/out"' && ok "$name"

name='macOS app install and upgrade'
fresh
release aarch64-apple-darwin
MOCK_OS=Darwin MOCK_ARCH=arm64; export MOCK_OS MOCK_ARCH
run_install --dir "$dest"
check "$name" '[ "$status" -eq 0 ]' &&
  run_install --dir "$dest" &&
  check "$name" '[ "$status" -eq 0 ] && [ -f "$dest/NsVim.app/Contents/Resources/nsh/bundle.nshc" ]' &&
  check "$name" '[ "$(ls -A "$dest")" = NsVim.app ]' &&
  check "$name" '[ "$("$root/home/.local/bin/nsvim")" = "nsvim $version" ]' &&
  check "$name" 'grep -q "run: $dest/NsVim.app/Contents/MacOS/nsvim \." "$root/out"' && ok "$name"

name='macOS upgrade from nsvim.app to NsVim.app'
fresh
release aarch64-apple-darwin old-app
MOCK_OS=Darwin MOCK_ARCH=arm64; export MOCK_OS MOCK_ARCH
run_install --dir "$dest"
check "$name" '[ "$status" -eq 0 ] && [ -d "$dest/nsvim.app" ]' &&
  release aarch64-apple-darwin &&
  run_install --dir "$dest" &&
  check "$name" '[ "$status" -eq 0 ] && [ "$(ls -A "$dest")" = NsVim.app ]' &&
  check "$name" '[ "$("$root/home/.local/bin/nsvim")" = "nsvim $version" ]' && ok "$name"

name='the command on PATH, in --bin-dir'
fresh
release x86_64-unknown-linux-gnu
status=0
env PATH="$root/bin:$root/mock:$PATH" HOME="$root/home" FIXTURES="$root/fixtures" MOCK_LATEST="$version" \
  TMPDIR="$root/tmp" "$shell" "$script" --dir "$dest" --bin-dir "$root/bin" > "$root/out" 2>&1 || status=$?
check "$name" '[ "$status" -eq 0 ] && [ "$("$root/bin/nsvim")" = "nsvim $version" ]' &&
  check "$name" '[ ! -e "$root/home/.local/bin" ] && grep -qx "Run: nsvim ." "$root/out"' && ok "$name"
rm -rf "${root:?}/bin"

name='a foreign nsvim command stays'
fresh
release x86_64-unknown-linux-gnu
mkdir -p "$root/home/.local/bin"
printf '#!/bin/sh\necho mine\n' > "$root/home/.local/bin/nsvim"
run_install --dir "$dest"
check "$name" '[ "$status" -eq 0 ] && [ "$(cat "$root/home/.local/bin/nsvim")" = "$(printf "#!/bin/sh\necho mine")" ]' &&
  check "$name" 'grep -q "is not from the nsvim installer" "$root/out"' && ok "$name"

name='an old link from setup is replaced'
fresh
release x86_64-unknown-linux-gnu
mkdir -p "$root/home/.local/bin"
ln -s "$root/old/nsvim" "$root/home/.local/bin/nsvim"
run_install --dir "$dest"
check "$name" '[ "$status" -eq 0 ] && [ ! -L "$root/home/.local/bin/nsvim" ] && [ "$("$root/home/.local/bin/nsvim")" = "nsvim $version" ]' && ok "$name"

name='invalid version downloads nothing'
fresh
run_install --version ../bad --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "Version must be" "$root/out" && [ ! -e "$dest" ]' && ok "$name"

name='unsupported architecture'
fresh
MOCK_ARCH=armv7l; export MOCK_ARCH
run_install --dir "$dest"
check "$name" '[ "$status" -ne 0 ] && grep -q "Unsupported CPU architecture" "$root/out"' && ok "$name"

# The archive that package-release.py writes installs: the two scripts agree on the layout.
name='package-release.py archive installs'
if command -v python3 >/dev/null; then
  fresh
  mkdir -p "$root/stage/build/nsh"
  printf '#!/bin/sh\necho packaged\n' > "$root/stage/build/nsvim"
  echo bytecode > "$root/stage/build/nsh/bundle.nshc"
  echo 'export {}' > "$root/stage/build/nsh/bundle.js"
  echo "{}" > "$root/stage/build/nsh/bundle.js.map"
  echo '{"id":"dev.nsvim.app"}' > "$root/stage/build/nsh/app.json"
  TMPDIR="$root/tmp" python3 "$here/package-release.py" --target x86_64-unknown-linux-gnu --version "$version" \
    --binary "$root/stage/build/nsvim" --nsh "$root/stage/build/nsh" --output "$root/fixtures" > "$root/out" 2>&1
  mv "$root/fixtures/nsvim-$version-x86_64-unknown-linux-gnu.tar.gz.sha256" "$root/fixtures/SHA256SUMS"
  run_install --dir "$dest"
  check "$name" '[ "$status" -eq 0 ] && [ "$("$dest/share/nsvim/nsvim")" = packaged ]' &&
    check "$name" '[ -f "$dest/share/nsvim/nsh/bundle.nshc" ] && [ -f "$dest/share/nsvim/nsh/bundle.js" ] && [ ! -e "$dest/share/nsvim/nsh/bundle.js.map" ]' &&
    check "$name" 'grep -q "\"name\":\"NsVim\"" "$dest/share/nsvim/nsh/app.json"' &&
    check "$name" 'grep -qx "StartupWMClass=dev.nsvim.app" "$dest/share/applications/nsvim.desktop"' &&
    check "$name" 'grep -qx "Icon=nsvim" "$dest/share/applications/nsvim.desktop"' && ok "$name"
else
  printf 'SKIP %s: python3 is not installed\n' "$name"
fi

if [ "$failures" -ne 0 ]; then
  printf '%s check(s) failed\n' "$failures"
  exit 1
fi
printf '%s\n' 'All installer tests passed.'
