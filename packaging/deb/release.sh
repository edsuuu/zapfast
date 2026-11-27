#!/usr/bin/env bash
# Releases the version in Cargo.toml from this fork: checks that main is
# committed and pushed with packaging/release-notes/vVERSION.md, then pushes
# the tag vVERSION. The Fork release workflow (.github/workflows/fork-release.yml)
# builds the .deb, makes the GitHub release and publishes the apt repository.
#
#   packaging/deb/release.sh
#
# To publish from this machine instead, with Docker and the signing key in
# ZAPFAST_APT_GNUPGHOME, run build-deb.sh and publish-apt.sh by hand.
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

git tag -a "$tag" -m "ZapFast ${version}"
git push -q "https://github.com/${repo}.git" "refs/tags/${tag}"
echo "pushed ${tag}; follow it at https://github.com/${repo}/actions/workflows/fork-release.yml"
