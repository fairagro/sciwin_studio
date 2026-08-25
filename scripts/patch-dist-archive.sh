#!/usr/bin/env bash
# Called once per target triple from .github/workflows/build-app.yml, after
# `dist build --artifacts=local` has already produced a correct archive +
# per-target dist-manifest.json for that triple.
#
# dist itself only ever compiles this crate with a plain `cargo build`
# (build-command is a Generic/JS-workspace-only feature; this is a Cargo
# workspace, so dist never runs `dx bundle`), so the archive dist just built
# contains a bare, unstyled sciwin_studio binary. This script builds the real
# no-admin-rights installer/app with `dx bundle` (AppImage on Linux, per-user
# NSIS on Windows, .app on macOS) and patches it into that same archive under
# the same filename, then updates the one checksum dist's manifest records
# for it so later `dist` steps (which read the manifest, not the file) stay
# consistent with what's actually in the archive.
set -euo pipefail

target="$1" # e.g. x86_64-unknown-linux-gnu
manifest="$2" # path to this target's dist-manifest.json

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# dist names archives "<pkg>-<target-triple>.<ext>" and uses that exact
# filename (basename) as the artifact's key in the manifest, so we don't need
# to know the package name or parse the manifest to find the file.
archive_path=$(find target/distrib -maxdepth 1 \( -name "*-${target}.tar.xz" -o -name "*-${target}.zip" \) | head -n1)
if [ -z "$archive_path" ]; then
  echo "patch-dist-archive.sh: no archive for target '$target' found under target/distrib" >&2
  exit 1
fi
archive_name=$(basename "$archive_path")
artifact_dir_name="${archive_name%.tar.xz}"
artifact_dir_name="${artifact_dir_name%.zip}"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

case "$target" in
  *windows*)
    dx bundle --desktop --profile dist --target "$target" --package-types nsis
    installer=$(find target/dx -iname '*.exe' -path '*/bundle/*' | head -n1)
    [ -n "$installer" ] || { echo "no NSIS installer found under target/dx" >&2; exit 1; }

    unpack="$stage/unpack"
    mkdir -p "$unpack"
    7z x "$archive_path" -o"$unpack" >/dev/null
    cp "$installer" "$unpack/sciwin_studio.exe"
    rm -f "$archive_path"
    (cd "$unpack" && 7z a "$OLDPWD/$archive_path" . >/dev/null)
    ;;
  *apple-darwin*)
    dx bundle --desktop --profile dist --target "$target" --package-types macos
    app=$(find target/dx -iname '*.app' -path '*/bundle/*' -maxdepth 6 | head -n1)
    [ -n "$app" ] || { echo "no .app bundle found under target/dx" >&2; exit 1; }

    unpack="$stage/unpack"
    mkdir -p "$unpack"
    tar -xf "$archive_path" -C "$unpack"
    cp -R "$app" "$unpack/$artifact_dir_name/SciWIn Studio.app"
    rm -f "$archive_path"
    tar -cJf "$archive_path" -C "$unpack" "$artifact_dir_name"
    ;;
  *linux*)
    dx bundle --desktop --profile dist --target "$target" --package-types appimage
    appimage=$(find target/dx -iname '*.AppImage' -path '*/bundle/*' | head -n1)
    [ -n "$appimage" ] || { echo "no AppImage found under target/dx" >&2; exit 1; }

    unpack="$stage/unpack"
    mkdir -p "$unpack"
    tar -xf "$archive_path" -C "$unpack"
    cp "$appimage" "$unpack/$artifact_dir_name/sciwin_studio"
    chmod +x "$unpack/$artifact_dir_name/sciwin_studio"
    rm -f "$archive_path"
    tar -cJf "$archive_path" -C "$unpack" "$artifact_dir_name"
    ;;
  *)
    echo "patch-dist-archive.sh: unrecognized target '$target'" >&2
    exit 1
    ;;
esac

# macOS has no sha256sum by default, only shasum
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -b "$archive_path"
else
  shasum -a 256 -b "$archive_path"
fi | sed "s#$archive_path#$archive_name#" > "$archive_path.sha256"
new_sum=$(cut -d' ' -f1 < "$archive_path.sha256")

tmp_manifest=$(mktemp)
jq --arg name "$archive_name" --arg sum "$new_sum" \
  '.artifacts[$name].checksums.sha256 = $sum' "$manifest" > "$tmp_manifest"
mv "$tmp_manifest" "$manifest"
