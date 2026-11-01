#!/bin/sh
# Build a minimal macOS app bundle around the release binary.
set -e
cd "$(dirname "$0")/.."
cargo build --release
APP=dist/1by1.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/presets" "$APP/Contents/Frameworks"
cp target/release/onebyone "$APP/Contents/MacOS/1by1"
cp assets/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cp vendor/projectm4/lib/libprojectM-4.4.1.8.dylib "$APP/Contents/Frameworks/"
cp vendor/projectm4/lib/libprojectM-4-playlist.4.1.8.dylib "$APP/Contents/Frameworks/"
ln -s libprojectM-4.4.1.8.dylib "$APP/Contents/Frameworks/libprojectM-4.4.dylib"
ln -s libprojectM-4-playlist.4.1.8.dylib "$APP/Contents/Frameworks/libprojectM-4-playlist.4.dylib"
cp vendor/projectm4/LICENSE.txt "$APP/Contents/Resources/projectM-LICENSE.txt"
if [ -d /opt/homebrew/share/projectM/presets ]; then
    cp -R /opt/homebrew/share/projectM/presets/. "$APP/Contents/Resources/presets/"
fi
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>1by1</string>
    <key>CFBundleDisplayName</key><string>1by1</string>
    <key>CFBundleIdentifier</key><string>de.example.onebyone</string>
    <key>CFBundleExecutable</key><string>1by1</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>0.1</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
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
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
echo "built $APP"
