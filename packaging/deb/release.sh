#!/usr/bin/env bash
# Releases the version in Cargo.toml from this fork: builds the .deb, makes
# the GitHub release vVERSION with it and the notes in
# packaging/release-notes/vVERSION.md, and publishes the apt repository.
# Run it on main, committed and pushed.
#
#   packaging/deb/release.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
repo=${ZAPFAST_APT_REPO:-edsuuu/zapfast}
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
tag="v${version}"
notes="packaging/release-notes/${tag}.md"
die() { echo "$*" >&2; exit 1; }
[ -f "$notes" ] || die "write $notes first"
[ -z "$(git status --porcelain --untracked-files=no)" ] || die "commit your changes first"
git fetch -q origin main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || die "push main first"

deb=$(packaging/deb/build-deb.sh | tail -1)
if gh release view "$tag" -R "$repo" > /dev/null 2>&1; then
  gh release upload "$tag" "$deb" -R "$repo" --clobber
else
  gh release create "$tag" "$deb" -R "$repo" --target "$(git rev-parse HEAD)" \
    --title "ZapFast ${version}" --notes-file "$notes"
fi
packaging/deb/publish-apt.sh "$deb"
