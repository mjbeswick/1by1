#!/bin/sh
# Build dist/OneByOne.app: release binary, Homebrew dylibs copied into Frameworks,
# install names rewritten to @rpath, ad hoc signed. Runs on macOS with Homebrew libs.
set -e
cd "$(dirname "$0")/.."
# A minimal FFmpeg built by build-ffmpeg-min.sh takes precedence over Homebrew's.
ALT=""
if [ -d "$PWD/.ffmpeg-min" ]; then
    ALT="$PWD/.ffmpeg-min"
    export PKG_CONFIG_PATH="$ALT/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi
swift build -c release
APP=dist/OneByOne.app
BIN=.build/release/OneByOne
FW="$APP/Contents/Frameworks"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$FW" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/OneByOne"
cp assets/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

# Walk Homebrew dependencies until none are left, copying each dylib once.
homebrew_deps() {
    otool -L "$1" | awk 'NR>1 {print $1}' | grep -E "^/opt/homebrew/${ALT:+|^$ALT/}" || true
}
queue="$BIN"
seen=""
while [ -n "$queue" ]; do
    next=""
    for f in $queue; do
        for dep in $(homebrew_deps "$f"); do
            real=$(realpath "$dep")
            name=$(basename "$real")
            case " $seen " in *" $name "*) continue ;; esac
            seen="$seen $name"
            cp "$real" "$FW/$name"
            chmod u+w "$FW/$name"
            install_name_tool -id "@rpath/$name" "$FW/$name"
            next="$next $FW/$name"
        done
    done
    queue="$next"
done

# Point every reference at the bundled copy.
for f in "$APP/Contents/MacOS/OneByOne" "$FW"/*.dylib; do
    for dep in $(homebrew_deps "$f"); do
        name=$(basename "$(realpath "$dep")")
        install_name_tool -change "$dep" "@rpath/$name" "$f"
    done
done
install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/OneByOne" 2>/dev/null || true

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>1by1</string>
    <key>CFBundleDisplayName</key><string>1by1</string>
    <key>CFBundleIdentifier</key><string>dev.onebyone.app</string>
    <key>CFBundleExecutable</key><string>OneByOne</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>0.2</string>
    <key>CFBundleVersion</key><string>2</string>
    <key>LSMinimumSystemVersion</key><string>14.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSPrincipalClass</key><string>NSApplication</string>
    <key>CFBundleDocumentTypes</key>
    <array>
      <dict>
        <key>CFBundleTypeName</key><string>Audio file</string>
        <key>CFBundleTypeRole</key><string>Viewer</string>
        <key>LSItemContentTypes</key>
        <array><string>public.audio</string></array>
      </dict>
    </array>
</dict>
</plist>
PLIST

# Drop symbol tables from the bundled libraries. Saves about 4 MB and changes no behaviour.
strip -x "$FW"/*.dylib "$APP/Contents/MacOS/OneByOne" 2>/dev/null || true

codesign --force --deep -s - "$APP"
echo "built $APP"
