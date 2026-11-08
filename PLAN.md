# 1by1 native macOS rewrite

## Goal

Native macOS app in Swift and AppKit. Folder is the queue. Drop the Milkdrop visualizer and Windows. Keep the app small.

## Status

Core port done. Settings pages and behaviors ported. Remaining gaps listed under Not yet applied.

## Done
- [x] Package.swift, system library module map, swift build green
- [x] App launches, main window, native menu bar (File, Play, View, Bookmarks, Help)
- [x] Folder tree (NSOutlineView), lazy children
- [x] Track list (NSTableView), natural order, full-row select
- [x] Status bar, title bar shows track name
- [x] AVFoundation decoder source, AVAudioEngine output
- [x] Play, stop/resume, space toggles
- [x] Seek slider (applies on release), volume, mute
- [x] Rewind/forward 5 s
- [x] Next/prev track, next/prev folder via remote
- [x] Gapless queue
- [x] Ogg Vorbis source (libvorbisfile), verified with a test file from libvorbisenc
- [x] Tracker source (libopenmpt), metadata from module
- [x] Repeat off/all/track, shuffle (random next row)
- [x] Playlist view (bottom pane), enqueue, play from playlist
- [x] Cue sheets, m3u/m3u8 loading
- [x] Bookmarks (30), history (10 recent folders)
- [x] Track info window with tag read and write (TagLib)
- [x] Find in list (toolbar), go to track
- [x] Settings window (subset, see below)
- [x] Big title view
- [x] HTTP remote (NWListener), same paths as before
- [x] Drag and drop files and folders
- [x] Packaging script with bundled dylibs
- [x] Rust code deleted
- [x] README rewritten for macOS

## Not yet applied

- Removed from the settings window because nothing reads them: Proxy (the app makes no outbound requests), No label edit, File info menu. Their values stay in the settings file.
- Not notarized (blocked on the user): this Mac has only an "Apple Development" identity, which cannot notarize distribution builds. Needed: a "Developer ID Application" certificate, plus notarytool credentials (Apple ID with app-specific password, or an App Store Connect API key). Gatekeeper rejects the app until then. The bundle loads only its own libraries (checked with DYLD_PRINT_LIBRARIES), so it is relocatable. Size is 6 MB. FFmpeg is a minimal decode-only build (`scripts/build-ffmpeg-min.sh`, output in `.ffmpeg-min/`).

## Verification

- `--selftest`, `--smoke` (AVFoundation, Vorbis, OpenMPT, FFmpeg formats), `--scan`, `--import-check`.
- Window at default size, resized, and compact view checked by screenshot.
- Ogg Vorbis checked with a file from libvorbisenc. Opus-in-CAF checked with a CAF file built from Ogg Opus packets: AVFoundation reads and plays it.
- Rust settings import checked against the real `~/.config/1by1/settings.toml`.

## Decisions

- Swift package at repo root. AppKit only. No SwiftUI.
- AVFoundation decodes most formats. libvorbisfile for Ogg Vorbis. libopenmpt for trackers. TagLib for tags.
- Playback stops when the folder changes, so list indexes stay valid.
- Settings in JSON under Application Support. TOML not read.
- Deployment target macOS 14. Homebrew libs target 26; the linker warns.
- Bundle runs on the build machine (ad hoc signed, Homebrew dylibs copied and rewritten).
