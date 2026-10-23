#!/usr/bin/env bash
# Xcode's build step: builds the client for the platform being built, with
# MoltenVK linked in, as the app's executable, and embeds OpenAL Soft, which
# the client loads at runtime. Xcode signs the app after this.
set -euo pipefail

case "$PLATFORM_NAME" in
    iphonesimulator) triple=aarch64-apple-ios-sim platform=ios-simulator ;;
    iphoneos) triple=aarch64-apple-ios platform=ios ;;
    *) echo "error: unsupported platform $PLATFORM_NAME" >&2; exit 1 ;;
esac
profile=debug
release=
if [ "$CONFIGURATION" = Release ]; then
    profile=release
    release=--release
fi

app="$TARGET_BUILD_DIR/$WRAPPER_NAME"
frameworks="$TARGET_BUILD_DIR/$FRAMEWORKS_FOLDER_PATH"
export PATH="$HOME/.cargo/bin:$PATH"
# Xcode points these at the iOS SDK, which would break the host-built build
# scripts (shaderc); cc picks each target's SDK itself.
unset SDKROOT LIBRARY_PATH

cd "$PROJECT_DIR/.."
# shaderc, built for the Mac, keeps using a real Vulkan SDK's copy when there
# is one; the app links MoltenVK as its Vulkan.
if [ -n "${VULKAN_SDK:-}" ]; then
    export SHADERC_LIB_DIR="$VULKAN_SDK/lib"
fi
VULKAN_SDK="$(python3 tools/fetch_moltenvk.py "$platform")"
export VULKAN_SDK
# Host-built C code (SteelMC's build-time downloads pull in aws-lc-sys) needs
# the Mac's SDK, which Xcode's script environment has no default for. cc and
# rustc ignore an SDKROOT naming another platform and find the iOS SDK for
# the target themselves.
host_sdk="$(xcrun --sdk macosx --show-sdk-path)"
cargo_build() {
    SDKROOT="$host_sdk" cargo build --target "$triple" ${release:+"$release"} "$@"
}
# steel-worldgen's build script reads the Minecraft data that steel-utils'
# build script downloads, without depending on it, so on a fresh checkout it
# can run first and fail. steel-utils goes first.
cargo_build -p steel-utils
cargo_build -p pomme-ios
mkdir -p "$app" "$frameworks"
# Copied only when it changed, so an unchanged build isn't re-signed.
if ! cmp -s "target/$triple/$profile/pomme-ios" "$app/$EXECUTABLE_NAME"; then
    cp "target/$triple/$profile/pomme-ios" "$app/$EXECUTABLE_NAME"
fi

python3 tools/build_openal.py "$platform" "$frameworks"
if [ -n "${EXPANDED_CODE_SIGN_IDENTITY:-}" ]; then
    codesign --force --sign "$EXPANDED_CODE_SIGN_IDENTITY" --timestamp=none \
        "$frameworks/soft_oal.framework"
fi
