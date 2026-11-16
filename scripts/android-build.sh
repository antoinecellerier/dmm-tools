#!/usr/bin/env bash
# Build the experimental Android APK of dmm-gui.
#
# Usage:
#   scripts/android-build.sh [--release] [--install]
#
#   --release   build optimised, as the published APK is; signed with the
#               debug key all the same (scripts/android-sign.sh signs a
#               published one)
#   --install   install the APK on the phone adb sees and start it
#
# What it needs is listed under "Building for Android" in
# docs/development.md; a missing piece is named and nothing is installed
# here. The one download is Gradle itself, on a first run, checked against
# the checksum below and kept in the user's cache folder.
#
# The Rust half is cross-compiled by rustup's toolchain (the only one with an
# Android target), found in ~/.cargo/bin, which this script alone puts on
# PATH: every other build of the workspace keeps the system's cargo.
set -euo pipefail

# The NDK android/app/build.gradle names too.
NDK_VERSION=27.2.12479018
GRADLE_VERSION=8.9
GRADLE_SHA256=d725d707bfabd4dfdc958c624003b3c80accc03f7037b5122c4b1d0ef15cecab
APP_ID=io.github.antoinecellerier.dmmtools

profile=debug
install=
for arg in "$@"; do
    case "$arg" in
        --release) profile=release ;;
        --install) install=1 ;;
        *)
            echo "usage: $0 [--release] [--install]" >&2
            exit 2
            ;;
    esac
done

repo=$(cd "$(dirname "$0")/.." && pwd)
export ANDROID_HOME=${ANDROID_HOME:-$HOME/Android/Sdk}
export JAVA_HOME=${DMM_ANDROID_JAVA_HOME:-/usr/lib/jvm/java-17-openjdk-amd64}
export PATH="$HOME/.cargo/bin:$PATH"

missing=()
[ -x "$HOME/.cargo/bin/rustup" ] || missing+=("rustup (https://rustup.rs), with: rustup target add aarch64-linux-android")
[ -x "$HOME/.cargo/bin/cargo-ndk" ] || missing+=("cargo-ndk: cargo install cargo-ndk")
[ -x "$JAVA_HOME/bin/java" ] || missing+=("a JDK 17 at $JAVA_HOME (or set DMM_ANDROID_JAVA_HOME)")
[ -d "$ANDROID_HOME/platforms/android-34" ] || missing+=("the SDK platform: sdkmanager 'platforms;android-34'")
[ -d "$ANDROID_HOME/build-tools/34.0.0" ] || missing+=("the SDK build tools: sdkmanager 'build-tools;34.0.0'")
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/$NDK_VERSION
[ -d "$ANDROID_NDK_HOME" ] || missing+=("the NDK: sdkmanager 'ndk;$NDK_VERSION'")
if [ ${#missing[@]} -gt 0 ]; then
    echo "Missing, under ANDROID_HOME=$ANDROID_HOME:" >&2
    printf '  - %s\n' "${missing[@]}" >&2
    exit 1
fi

# Gradle, at the version the Android plugin in android/settings.gradle
# pairs with. The system's may be far older.
cache=${XDG_CACHE_HOME:-$HOME/.cache}/dmm-tools
gradle=$cache/gradle-$GRADLE_VERSION/bin/gradle
if [ ! -x "$gradle" ]; then
    mkdir -p "$cache"
    zip=$cache/gradle-$GRADLE_VERSION-bin.zip
    curl -sSfL -o "$zip.tmp" "https://services.gradle.org/distributions/gradle-$GRADLE_VERSION-bin.zip"
    echo "$GRADLE_SHA256  $zip.tmp" | sha256sum --check --quiet
    mv "$zip.tmp" "$zip"
    unzip -q -o "$zip" -d "$cache"
fi

cd "$repo"
# dmm-android asks for dmm-gui without its default features, so the update
# check and its TLS stack stay out.
if [ "$profile" = release ]; then
    cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release -p dmm-android
    "$gradle" --project-dir android assembleRelease
else
    cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p dmm-android
    "$gradle" --project-dir android assembleDebug
fi
apk=android/app/build/outputs/apk/$profile/app-$profile.apk
echo "Built $apk"

if [ -n "$install" ]; then
    adb install -r "$apk"
    adb shell am start -n "$APP_ID/.PermissionActivity"
fi
