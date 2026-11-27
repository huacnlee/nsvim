#!/usr/bin/env python3
"""Pack a release build of nsvim into the archive that install.sh and install.ps1 read.

The program reads its interface (App.tsx, compiled by `nsh build`) from `nsh/bundle.nshc`.
On Linux and Windows, `nsh/` is next to the executable. On macOS, it is in the app bundle's
`Contents/Resources/nsh`. Each archive also gets a `.sha256` file. The release workflow joins
them into one `SHA256SUMS`.

The window updates itself with NSH's updater (`nsh.json` `"updater"`). It reads
`nsh/assets/nsh-updater.json`, which `nsh package` writes and this script writes the same way.
NSH's updater takes an archive that holds one folder, the app, named
`NsVim-<version>-<os>-<arch>`. So each platform also gets that archive, which
`nsh update-manifest` lists in `latest.json`. The installers keep reading the first archive.
"""
import argparse
import hashlib
import json
import plistlib
import re
import shutil
import struct
import sys
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    "aarch64-apple-darwin": "macos",
    "x86_64-apple-darwin": "macos",
    "x86_64-unknown-linux-gnu": "linux",
    "aarch64-unknown-linux-gnu": "linux",
    "x86_64-pc-windows-msvc": "windows",
}
APP_ID = "dev.nsvim.app"
# NSH 0.7 needs macOS 14 (`nsh package` writes the same minimum).
MACOS_MINIMUM = "14.0"
# NSH's names of the platforms (`nsh_updater::platform()`), for the updater's archives.
UPDATER_PLATFORMS = {
    "aarch64-apple-darwin": "macos-aarch64",
    "x86_64-apple-darwin": "macos-x86_64",
    "x86_64-unknown-linux-gnu": "linux-x86_64",
    "aarch64-unknown-linux-gnu": "linux-aarch64",
    "x86_64-pc-windows-msvc": "windows-x86_64",
}


def updater_config(version):
    """What `nsh package` writes as `nsh-updater.json` (`nsh_updater::Config`), from nsh.json."""
    app = json.loads((ROOT / "nsh.json").read_text())
    updater = app.get("updater")
    if not updater:
        return None
    config = {"id": app.get("id", APP_ID), "version": version, "name": app.get("name", "NsVim")}
    for key in ("github", "url", "channels", "publicKey"):
        if key in updater:
            config[key] = updater[key]
    config["build"] = app.get("build", 1)
    return json.dumps(config, separators=(",", ":"))


def interface(nsh_dir, stage, version):
    """Copy the output of `nsh build` to `stage`, without the source maps. The window reads
    `bundle.nshc`. The terminal editor (`nsvim` in a terminal) reads `bundle.js`."""
    if not (nsh_dir / "bundle.nshc").is_file():
        raise SystemExit(f"{nsh_dir} has no bundle.nshc: run `nsh build --release -o {nsh_dir}`")
    if not (nsh_dir / "bundle.js").is_file():
        raise SystemExit(f"{nsh_dir} has no bundle.js, which the terminal editor reads")
    shutil.copytree(nsh_dir, stage, ignore=shutil.ignore_patterns("*.js.map"))
    # The runner reads the window title from `name`. `nsh build` writes only the id.
    app = json.loads((stage / "app.json").read_text()) if (stage / "app.json").is_file() else {}
    app.setdefault("id", APP_ID)
    app.setdefault("name", "NsVim")
    (stage / "app.json").write_text(json.dumps(app, separators=(",", ":")))
    config = updater_config(version)
    if config:
        (stage / "assets").mkdir(exist_ok=True)
        (stage / "assets/nsh-updater.json").write_bytes(config.encode())


def desktop_entry():
    """The Linux desktop entry, with the same changes as the first-run setup makes."""
    text = (ROOT / "assets/editor.desktop").read_text()
    for old, new in (
        ("Exec=editor %F", "Exec=nsvim %F"),
        ("Name=editor", "Name=NsVim"),
        ("Icon=editor", "Icon=nsvim"),
        ("StartupWMClass=editor", f"StartupWMClass={APP_ID}"),
    ):
        if old not in text:
            raise SystemExit(f"assets/editor.desktop has no line {old!r}")
        text = text.replace(old, new)
    return text


def icns(path):
    """Make the macOS icon from assets/editor.svg with rsvg-convert and iconutil."""
    with tempfile.TemporaryDirectory(prefix="nsvim-icon-") as temporary:
        iconset = Path(temporary) / "nsvim.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
                pixels = str(size * scale)
                subprocess.run(
                    ["rsvg-convert", "-w", pixels, "-h", pixels, "-o", str(iconset / name), str(ROOT / "assets/editor.svg")],
                    check=True,
                )
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(path)], check=True)


def windows_icon(exe, ico):
    """Embed the icon file `ico` in `exe` as icon group 1, the one Explorer, the Start menu
    shortcut and the taskbar show. Uses Windows' resource update API, so it runs on Windows only.
    """
    import ctypes
    from ctypes import wintypes

    data = ico.read_bytes()
    _, kind, count = struct.unpack_from("<HHH", data)
    if kind != 1 or count == 0:
        raise SystemExit(f"{ico} is not an icon file")
    images, group = [], struct.pack("<HHH", 0, 1, count)
    for index in range(count):
        width, height, colors, _, planes, bits, size, offset = struct.unpack_from("<BBBBHHII", data, 6 + 16 * index)
        images.append(data[offset : offset + size])
        group += struct.pack("<BBBBHHIH", width, height, colors, 0, planes, bits, size, index + 1)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.BeginUpdateResourceW.restype = wintypes.HANDLE
    kernel32.BeginUpdateResourceW.argtypes = [wintypes.LPCWSTR, wintypes.BOOL]
    kernel32.UpdateResourceW.argtypes = [wintypes.HANDLE, ctypes.c_void_p, ctypes.c_void_p, wintypes.WORD, ctypes.c_void_p, wintypes.DWORD]
    kernel32.EndUpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.BOOL]
    update = kernel32.BeginUpdateResourceW(str(exe), False)
    if not update:
        raise ctypes.WinError(ctypes.get_last_error())
    RT_ICON, RT_GROUP_ICON, LANGUAGE = 3, 14, 0
    ok = all(
        kernel32.UpdateResourceW(update, RT_ICON, index + 1, LANGUAGE, image, len(image))
        for index, image in enumerate(images)
    ) and kernel32.UpdateResourceW(update, RT_GROUP_ICON, 1, LANGUAGE, group, len(group))
    if not kernel32.EndUpdateResourceW(update, not ok) or not ok:
        raise ctypes.WinError(ctypes.get_last_error())


def macos_app(stage, binary, nsh_dir, version):
    app = stage / "NsVim.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    (contents / "Resources").mkdir()
    shutil.copy2(binary, contents / "MacOS/nsvim")
    (contents / "MacOS/nsvim").chmod(0o755)
    interface(nsh_dir, contents / "Resources/nsh", version)
    icns(contents / "Resources/nsvim.icns")
    with (contents / "Info.plist").open("wb") as stream:
        plistlib.dump(
            {
                "CFBundleDevelopmentRegion": "en",
                "CFBundleIdentifier": APP_ID,
                "CFBundleName": "NsVim",
                "CFBundleDisplayName": "NsVim",
                "CFBundleExecutable": "nsvim",
                "CFBundleIconFile": "nsvim.icns",
                "CFBundlePackageType": "APPL",
                "CFBundleShortVersionString": version,
                "CFBundleVersion": version.split("-")[0],
                "LSMinimumSystemVersion": MACOS_MINIMUM,
                "LSApplicationCategoryType": "public.app-category.developer-tools",
                "NSHighResolutionCapable": True,
                "NSPrincipalClass": "NSApplication",
            },
            stream,
        )
    # An ad hoc signature keeps the executable valid on Apple silicon. A Developer ID signature
    # and notarization need the release owner's certificate.
    subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)


def owner(info):
    """Archive entries belong to root, not to the user of the build machine."""
    info.uid = info.gid = 0
    info.uname = info.gname = "root"
    return info


def updater_layout(platform):
    """The one folder of the updater's archive, and the entries of the stage it takes. macOS: the
    app bundle. Linux: `nsvim/`, as the install script installs `share/nsvim`, without
    `packaging/`. Windows: `nsvim/`, the folder the install script installs."""
    if platform == "macos":
        return "", lambda name: name.endswith(".app")
    if platform == "linux":
        return "nsvim", lambda name: name != "packaging"
    return "nsvim", lambda name: True


def write(archive, stage, root="", keep=lambda name: True):
    """Pack the entries of `stage` that `keep` takes into `archive`, under the folder `root`
    ("": at the top of the archive)."""
    entries = [path for path in sorted(stage.iterdir()) if keep(path.name)]
    under = lambda name: f"{root}/{name}" if root else name
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as bundle:
            for entry in entries:
                for path in [entry] if entry.is_file() else sorted(entry.rglob("*")):
                    if path.is_file():
                        bundle.write(path, under(path.relative_to(stage).as_posix()))
        return
    with tarfile.open(archive, "w:gz", format=tarfile.PAX_FORMAT) as bundle:
        for path in entries:
            bundle.add(path, arcname=under(path.name), filter=owner)


def package(binary, tui_binary, nsh_dir, output, version, target):
    platform = TARGETS[target]
    output.mkdir(parents=True, exist_ok=True)
    extension = "zip" if platform == "windows" else "tar.gz"
    archive = output / f"nsvim-{version}-{target}.{extension}"
    with tempfile.TemporaryDirectory(prefix="nsvim-package-") as temporary:
        stage = Path(temporary)
        if platform == "macos":
            macos_app(stage, binary, nsh_dir, version)
        elif platform == "linux":
            shutil.copy2(binary, stage / "nsvim")
            (stage / "nsvim").chmod(0o755)
            interface(nsh_dir, stage / "nsh", version)
            (stage / "packaging").mkdir()
            (stage / "packaging/nsvim.desktop").write_text(desktop_entry())
            shutil.copy2(ROOT / "assets/editor.svg", stage / "packaging/nsvim.svg")
        else:
            shutil.copy2(binary, stage / "nsvim.exe")
            # The terminal editor: a console program, because nsvim.exe gets no console.
            if tui_binary is None:
                raise SystemExit("Windows needs --tui-binary, the release build of nsvim-tui.exe")
            shutil.copy2(tui_binary, stage / "nsvim-tui.exe")
            interface(nsh_dir, stage / "nsh", version)
            # The window, taskbar and Alt+Tab icon: the runner reads nsh\icon.ico.
            shutil.copy2(ROOT / "assets/nsvim.ico", stage / "nsh/icon.ico")
            # The executable's own icon, which Explorer and the Start menu shortcut show.
            if sys.platform == "win32":
                windows_icon(stage / "nsvim.exe", ROOT / "assets/nsvim.ico")
                windows_icon(stage / "nsvim-tui.exe", ROOT / "assets/nsvim.ico")
        for name in ("LICENSE", "README.md"):
            shutil.copy2(ROOT / name, stage / name)
        write(archive, stage)
        if updater_config(version):
            root, keep = updater_layout(platform)
            update = output / f"NsVim-{version}-{UPDATER_PLATFORMS[target]}.{extension}"
            write(update, stage, root, keep)
            print(update)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    # The Unix installers read SHA256SUMS. Keep LF line ends, also from the Windows job.
    archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n", encoding="ascii", newline="\n")
    print(archive)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--version", required=True)
    parser.add_argument("--binary", type=Path, required=True, help="the release build of nsvim")
    parser.add_argument("--tui-binary", type=Path, help="the release build of nsvim-tui (Windows only)")
    parser.add_argument("--nsh", type=Path, required=True, help="the output folder of `nsh build --release`")
    parser.add_argument("--output", type=Path, default=Path("dist"))
    args = parser.parse_args()
    if not args.binary.is_file():
        parser.error(f"binary not found: {args.binary}")
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", args.version):
        parser.error("version must be a release version without a leading v, such as 0.4.0")
    tui_binary = args.tui_binary.resolve() if args.tui_binary else None
    package(args.binary.resolve(), tui_binary, args.nsh.resolve(), args.output.resolve(), args.version, args.target)
