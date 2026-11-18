#!/usr/bin/env bash
# Builds target/deb/dist/zapfast_<version>_amd64.deb from this checkout.
#
# The binary is built in Debian 12 (glibc 2.36), as upstream's releases are,
# so the package installs on Debian 12+, Ubuntu 23.04+ and their derivatives.
# It carries the same name, files and dependencies as upstream's .deb, so it
# upgrades an installed upstream package in place; a higher version wins.
#
#   packaging/deb/build-deb.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
arch=amd64
[ -n "$version" ] || { echo "no version in Cargo.toml" >&2; exit 1; }

packaging/deb/in-container.sh cargo build --locked --release
binary=target/deb/release/zapfast
[ -x "$binary" ] || { echo "the build left no $binary" >&2; exit 1; }

stage=target/deb/pkg/zapfast_${version}_${arch}
dist=target/deb/dist
rm -rf "$stage"
mkdir -p "$stage/DEBIAN" "$dist"
install -Dm755 "$binary" "$stage/usr/bin/zapfast"
install -Dm644 packaging/applications/zapfast.desktop "$stage/usr/share/applications/zapfast.desktop"
install -Dm644 packaging/icons/zapfast.svg "$stage/usr/share/icons/hicolor/scalable/apps/zapfast.svg"
install -Dm644 LICENSE "$stage/usr/share/licenses/zapfast/LICENSE"
install -Dm644 README.md "$stage/usr/share/doc/zapfast/README.md"
install -Dm644 contrib/omarchy/zapfast.json.tpl "$stage/usr/share/zapfast/omarchy/zapfast.json.tpl"
install -Dm755 contrib/omarchy/zapfast-theme "$stage/usr/share/zapfast/omarchy/zapfast-theme"

# The libraries ZapFast opens while running and the ones it links, as in
# upstream's native-packages.yaml; libc at the version the build needs.
depends=(
  "libc6 (>= 2.36)"
  libgl1 libegl1 libxkbcommon0 libxkbcommon-x11-0
  libwayland-client0 libwayland-cursor0 libwayland-egl1
  libx11-6 libx11-xcb1 libxcursor1 libxi6 libxrandr2
  "libasound2t64 | libasound2"
  "libpipewire-0.3-0t64 | libpipewire-0.3-0"
  libgcc-s1 libstdc++6
)
joined=$(printf '%s, ' "${depends[@]}")
cat > "$stage/DEBIAN/control" <<CONTROL
Package: zapfast
Version: ${version}
Architecture: ${arch}
Maintainer: edsuuu <111332935+edsuuu@users.noreply.github.com>
Installed-Size: $(du -sk --exclude=DEBIAN "$stage" | cut -f1)
Depends: ${joined%, }
Provides: fastsapp
Conflicts: fastsapp
Replaces: fastsapp
Section: net
Priority: optional
Homepage: https://github.com/edsuuu/zapfast
Description: Fast native WhatsApp client
 ZapFast is a native WhatsApp client written in Rust and egui. This build
 comes from the edsuuu/zapfast fork and updates through its apt repository,
 https://edsuuu.github.io/zapfast/apt.
CONTROL

# Whatever the umask, directories go out as 755.
find "$stage" -type d -exec chmod 755 {} +
deb="$dist/zapfast_${version}_${arch}.deb"
dpkg-deb --build --root-owner-group -Zxz "$stage" "$deb" > /dev/null
dpkg-deb --info "$deb" | sed -n 's/^ \(Package\|Version\|Architecture\|Installed-Size\):/\1:/p'
echo "$deb"
