#!/usr/bin/env bash
set -euo pipefail

target="${1:?Usage: bash scripts/package-macos.sh RUST_TARGET}"
case "$target" in
  aarch64-apple-darwin) architecture=arm64 ;;
  x86_64-apple-darwin) architecture=x86_64 ;;
  *) echo "Unsupported macOS target: $target" >&2; exit 1 ;;
esac
binary="target/$target/release/e2xf"
test -f "$binary"
actual_architecture="$(lipo -archs "$binary")"
test "$actual_architecture" = "$architecture"
output="dist/macos/$architecture"
app="$output/e2xf.app"
contents="$app/Contents"
icons="$output/e2xf.iconset"
mkdir -p "$contents/MacOS" "$contents/Resources" "$icons"
install -m 755 "$binary" "$contents/MacOS/e2xf"
cp packaging/macos/Info.plist "$contents/Info.plist"
version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$contents/Info.plist"
assets=macos/Runner/Assets.xcassets/AppIcon.appiconset
for size in 16 32 128 256 512; do
  double_size=$((size * 2))
  cp "$assets/app_icon_$size.png" "$icons/icon_$size"x"$size.png"
  cp "$assets/app_icon_$double_size.png" "$icons/icon_$size"x"$size@2x.png"
done
iconutil -c icns "$icons" -o "$contents/Resources/e2xf.icns"
codesign --force --sign - "$app"
echo "$app"
