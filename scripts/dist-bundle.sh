#!/usr/bin/env bash
# Invoked by `dist` (cargo-dist) as the per-target build-command.
#
# `dx bundle` alone only produces installable app bundles (Linux: an FHS
# usr/bin+usr/lib staging tree meant for .deb/.rpm/.AppImage; Windows: MSI/NSIS
# installers only). None of that is runnable in place, and dist itself only
# ever archives a single loose binary file next to Cargo.toml - so without this
# script, dist ends up shipping a bare exe with no way to find its assets,
# which is why release downloads render unstyled. We build the no-admin-rights
# installer format for each OS with dx, then hand dist a file named after the
# package binary so its normal single-file archiving picks up something that
# actually works standalone.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Placeholder so `include = ["SciWIn Studio.app"]` in dist-workspace.toml has
# something to archive on every platform; only macOS overwrites it for real.
mkdir -p "SciWIn Studio.app"

# --profile alone decides both the cargo profile and, since dx derives
# "is this a release build" (which gates asset-root baking/hashing) from the
# resolved profile rather than the separate --release flag, that dist (which
# inherits release) is treated as a release build too.
case "${CARGO_DIST_TARGET:-}" in
  *windows*)
    dx bundle --desktop --profile dist --package-types nsis
    installer=$(find target/dx -iname '*.exe' -path '*/bundle/*' | head -n1)
    cp "$installer" sciwin_studio.exe
    ;;
  *apple-darwin*)
    dx bundle --desktop --profile dist --package-types macos
    app=$(find target/dx -iname '*.app' -path '*/bundle/*' -maxdepth 6 | head -n1)
    rm -rf "SciWIn Studio.app"
    cp -R "$app" "SciWIn Studio.app"
    cp "SciWIn Studio.app/Contents/MacOS/sciwin_studio" sciwin_studio
    ;;
  *linux*)
    dx bundle --desktop --profile dist --package-types appimage
    appimage=$(find target/dx -iname '*.AppImage' -path '*/bundle/*' | head -n1)
    cp "$appimage" sciwin_studio
    chmod +x sciwin_studio
    ;;
  *)
    echo "dist-bundle.sh: unrecognized CARGO_DIST_TARGET '${CARGO_DIST_TARGET:-}'" >&2
    exit 1
    ;;
esac
