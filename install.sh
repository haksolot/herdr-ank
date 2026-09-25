#!/bin/sh
# The manifest's [[build]] (ADR-599b6f424271): puts the release binary of this
# platform in ./bin/herdr-ank, checked against SHA256SUMS, and falls back to
# cargo build --release only when that fails and cargo is present.
set -eu

cd "$(dirname "$0")"

version=$(sed -n 's/^version *= *"\(.*\)".*/\1/p' herdr-plugin.toml | sed -n 1p)
base=${HERDR_ANK_RELEASE_BASE:-https://github.com/haksolot/herdr-ank/releases/download/v$version}
platform=$(uname -sm)

case $platform in
"Linux x86_64") target=x86_64-unknown-linux-musl ;;
"Linux aarch64" | "Linux arm64") target=aarch64-unknown-linux-musl ;;
"Darwin x86_64") target=x86_64-apple-darwin ;;
"Darwin arm64" | "Darwin aarch64") target=aarch64-apple-darwin ;;
*) target= ;;
esac

fetch() {
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL -o "$2" "$1"
	elif command -v wget >/dev/null 2>&1; then
		wget -q -O "$2" "$1"
	else
		echo "herdr-ank: neither curl nor wget is on PATH" >&2
		return 1
	fi
}

sha256() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{print $1}'
	elif command -v shasum >/dev/null 2>&1; then
		shasum -a 256 "$1" | awk '{print $1}'
	else
		echo "herdr-ank: neither sha256sum nor shasum is on PATH" >&2
		return 1
	fi
}

# Downloads, checks and unpacks the release archive; returns 1 on any failure.
download() {
	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT
	fetch "$url" "$tmp/$asset" || return 1
	fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" || return 1
	expected=$(awk -v f="$asset" '$2 == f || $2 == "*" f {print $1}' "$tmp/SHA256SUMS")
	actual=$(sha256 "$tmp/$asset") || return 1
	if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
		echo "herdr-ank: $asset does not match its sum in $base/SHA256SUMS" >&2
		return 1
	fi
	tar -xzf "$tmp/$asset" -C "$tmp" herdr-ank || return 1
	mkdir -p bin
	cp "$tmp/herdr-ank" bin/herdr-ank
	chmod 755 bin/herdr-ank
}

if [ -n "$target" ]; then
	asset=herdr-ank-$version-$target.tar.gz
	url=$base/$asset
	if download; then
		exit 0
	fi
	tried="$url could not be downloaded and checked"
else
	tried="no release binary for platform '$platform'"
fi

if ! command -v cargo >/dev/null 2>&1; then
	echo "herdr-ank: $tried, and cargo is not on PATH to build from source" >&2
	exit 1
fi
echo "herdr-ank: $tried; building with cargo" >&2
cargo build --release
mkdir -p bin
cp target/release/herdr-ank bin/herdr-ank
