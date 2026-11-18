#!/usr/bin/env bash
# Publishes a .deb as this fork's apt repository on GitHub Pages:
# https://edsuuu.github.io/zapfast/apt, a flat repository signed with the
# fork's own key, as the MagoAgent one is.
#
#   packaging/deb/publish-apt.sh target/deb/dist/zapfast_<version>_amd64.deb
#
# The gh-pages branch is rebuilt from nothing each time, so only the latest
# version is served and the branch never grows. The signing key lives in
# ZAPFAST_APT_GNUPGHOME (~/.config/zapfast-apt/gnupg), outside the
# repository; the first run creates it. Back that folder up: losing it means
# every installed copy must trust a new key.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
deb=$(realpath "${1:?usage: publish-apt.sh path/to/zapfast_VERSION_amd64.deb}")
[ -f "$deb" ] || { echo "no such file: $deb" >&2; exit 1; }
repo=${ZAPFAST_APT_REPO:-edsuuu/zapfast}
owner=${repo%%/*}
name=${repo#*/}
base="https://${owner}.github.io/${name}"
export GNUPGHOME=${ZAPFAST_APT_GNUPGHOME:-$HOME/.config/zapfast-apt/gnupg}
uid="ZapFast apt (${repo}) <111332935+edsuuu@users.noreply.github.com>"

mkdir -p "$GNUPGHOME"
chmod 700 "$GNUPGHOME"
if ! gpg --batch --list-secret-keys "$uid" > /dev/null 2>&1; then
  echo "creating the signing key in $GNUPGHOME (back it up)" >&2
  gpg --batch --passphrase '' --quick-gen-key "$uid" rsa4096 sign never
fi
key=$(gpg --batch --list-secret-keys --with-colons "$uid" | awk -F: '/^fpr/ {print $10; exit}')

out="$root/target/deb/pages"
rm -rf "$out"
mkdir -p "$out/apt"
cp "$deb" "$out/apt/"
cd "$out/apt"
dpkg-scanpackages --multiversion . /dev/null 2> /dev/null > Packages
gzip -9kn Packages
{
  echo "Origin: ZapFast (${repo})"
  echo "Label: ZapFast"
  echo "Suite: stable"
  echo "Architectures: amd64"
  echo "Date: $(LC_ALL=C date -Ru)"
  echo "Description: ZapFast from the ${repo} fork"
  for sum in MD5Sum:md5sum SHA1:sha1sum SHA256:sha256sum SHA512:sha512sum; do
    echo "${sum%%:*}:"
    for file in Packages Packages.gz; do
      printf ' %s %s %s\n' "$(${sum#*:} "$file" | cut -d' ' -f1)" "$(stat -c%s "$file")" "$file"
    done
  done
} > Release
gpg --batch --yes --local-user "$key" --clearsign -o InRelease Release
gpg --batch --yes --local-user "$key" -abs -o Release.gpg Release
gpg --batch --export "$key" > zapfast.gpg
gpg --batch --armor --export "$key" > zapfast.asc
# Check what apt will check before anything goes out.
gpgv --keyring "$out/apt/zapfast.gpg" InRelease 2> /dev/null
gpgv --keyring "$out/apt/zapfast.gpg" Release.gpg Release 2> /dev/null

version=$(dpkg-deb --field "$deb" Version)
cd "$out"
touch .nojekyll
cat > index.html <<HTML
<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ZapFast pelo apt</title>
<style>
  :root { color-scheme: light dark; --fg: #0a0a0a; --bg: #f7f5f3; --code: #ffffff; --accent: #008069; }
  @media (prefers-color-scheme: dark) { :root { --fg: #fafafa; --bg: #161717; --code: #242626; --accent: #21c063; } }
  body { margin: 0; padding: 32px 16px; background: var(--bg); color: var(--fg);
         font: 16px/1.5 system-ui, sans-serif; }
  main { max-width: 760px; margin: 0 auto; }
  h1 { color: var(--accent); margin-top: 0; }
  pre { background: var(--code); padding: 12px 14px; border-radius: 8px; overflow-x: auto; }
  a { color: var(--accent); }
</style>
</head>
<body>
<main>
<h1>ZapFast pelo apt</h1>
<p>Versão ${version}, do fork <a href="https://github.com/${repo}">${repo}</a>, para Debian 12+, Ubuntu 23.04+ e derivados (amd64).</p>
<pre>curl -fsSL ${base}/apt/zapfast.gpg | sudo tee /usr/share/keyrings/zapfast.gpg &gt; /dev/null
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/zapfast.gpg] ${base}/apt ./" \\
  | sudo tee /etc/apt/sources.list.d/zapfast.list
sudo apt update &amp;&amp; sudo apt install zapfast</pre>
<p>Instala por cima do .deb oficial. Atualizar: <code>sudo apt update &amp;&amp; sudo apt upgrade</code>.
Sem o repositório: <a href="apt/$(basename "$deb")">$(basename "$deb")</a>.</p>
</main>
</body>
</html>
HTML
git init -q -b gh-pages
git add -A
git -c user.name="$(git -C "$root" config user.name)" -c user.email="$(git -C "$root" config user.email)" \
  commit -qm "apt: zapfast ${version}"
git push -qf "https://github.com/${repo}.git" gh-pages
cd "$root"

# Serve the branch on Pages the first time.
if ! gh api "repos/${repo}/pages" > /dev/null 2>&1; then
  gh api -X POST "repos/${repo}/pages" -f 'source[branch]=gh-pages' -f 'source[path]=/' > /dev/null
fi
echo "published zapfast ${version} at ${base}/apt"
