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

Stored in settings and shown in the settings window, but no effect yet:
- Folders in list (show subfolder rows in the track list).
- Scroll-wheel: volume works over the transport line. Folder tree and lists are not changed.
- Tab moves between toolbar and lists.
- Tree style "explorer", no label edit, file info menu.
- Skip lists in search, auto search directories, continue in focused list.
- Always scan tags (folders are scanned in the background anyway).
- Proxy (the app makes no outbound requests, so this has no use yet).
- Themes: the Rust build's named themes are not in settings. The app follows the system light and dark appearance.

Known limits:
- Ogg Vorbis verified only with a file encoded by libvorbisenc here.
- Layout verified by screenshot at default size. Resizing and the compact view are not checked.
- MKV, WebM and Opus-in-CAF are unsupported. FFmpeg would add about 40 MB.
- Settings from the Rust build (settings.toml, bookmarks, resume points) are not imported.

## Decisions

- Swift package at repo root. AppKit only. No SwiftUI.
- AVFoundation decodes most formats. libvorbisfile for Ogg Vorbis. libopenmpt for trackers. TagLib for tags.
- Playback stops when the folder changes, so list indexes stay valid.
- Settings in JSON under Application Support. TOML not read.
- Deployment target macOS 14. Homebrew libs target 26; the linker warns.
- Bundle runs on the build machine (ad hoc signed, Homebrew dylibs copied and rewritten).
