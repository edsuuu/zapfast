#!/usr/bin/env bash
# Runs a command inside the Debian 12 build image, with the checkout at /src
# and Cargo's download cache kept between runs in ~/.cache/zapfast-build.
# Builds the image first when it is missing. ZAPFAST_GIPHY_KEY, when set,
# goes along so the build carries a GIF search key.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cache=${ZAPFAST_BUILD_CACHE:-$HOME/.cache/zapfast-build}
mkdir -p "$cache/registry" "$cache/git"
image=${ZAPFAST_BUILD_IMAGE:-zapfast-deb-build}
if ! docker image inspect "$image" > /dev/null 2>&1; then
  docker build -t "$image" "$root/packaging/deb" >&2
fi
exec docker run --rm \
  --user "$(id -u):$(id -g)" \
  -e HOME=/tmp \
  -e CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/src/target/deb}" \
  ${ZAPFAST_GIPHY_KEY:+-e ZAPFAST_GIPHY_KEY} \
  -v "$root:/src" \
  -v "$cache/registry:/opt/cargo/registry" \
  -v "$cache/git:/opt/cargo/git" \
  -w /src \
  "$image" "$@"
