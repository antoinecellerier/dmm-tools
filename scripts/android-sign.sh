#!/usr/bin/env bash
# Sign an APK scripts/android-build.sh --release built with the project's
# key, for publishing.
#
# Usage:
#   scripts/android-sign.sh <in.apk> <out.apk>
#
# Environment:
#   DMM_ANDROID_KEYSTORE            the PKCS12 keystore file
#   DMM_ANDROID_KEYSTORE_PASSWORD   its password
#
# The signature the build put on the APK is replaced. The result is refused
# unless its certificate is the one android/signing-cert.sha256 names: an
# update installs only over a copy signed with the same key, so a published
# APK signed with any other would strand everyone who has the application.
set -euo pipefail

if [ $# -ne 2 ]; then
    echo "usage: $0 <in.apk> <out.apk>" >&2
    exit 2
fi
in=$1
out=$2
: "${DMM_ANDROID_KEYSTORE:?the keystore file to sign with}"
: "${DMM_ANDROID_KEYSTORE_PASSWORD:?the password of the keystore}"

repo=$(cd "$(dirname "$0")/.." && pwd)
tools=${ANDROID_HOME:-$HOME/Android/Sdk}/build-tools/34.0.0
expected=$(tr -d '[:space:]' < "$repo/android/signing-cert.sha256")

# Gradle aligned the file. Aligning it again here could undo that for the
# library, so this only checks.
"$tools/zipalign" -c 4 "$in"

"$tools/apksigner" sign \
    --ks "$DMM_ANDROID_KEYSTORE" --ks-pass env:DMM_ANDROID_KEYSTORE_PASSWORD \
    --out "$out" "$in"

actual=$("$tools/apksigner" verify --print-certs "$out" \
    | sed -n 's/^Signer #1 certificate SHA-256 digest: //p')
if [ "$actual" != "$expected" ]; then
    rm -f "$out"
    echo "signed with certificate ${actual:-<none>}," >&2
    echo "not $expected (android/signing-cert.sha256)" >&2
    exit 1
fi
echo "Signed $out"
