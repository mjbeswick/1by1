# 1by1 native macOS rewrite

## Goal

Native macOS app in Swift and AppKit. Folder is the queue. Drop the Milkdrop visualizer and Windows. Keep the app small.

## Status

Phases 1 to 5 checklist is done as listed below. Items under Deferred are not done.

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

## Deferred (not done)
- [ ] Settings: only remote, tree, shuffle, repeat. Rust build had eleven pages (themes, fonts, hotkeys, list styles, decoder options, logging, webserver options, and more).
- [ ] Find files (F3) and Explore folder (Ctrl+X) dialogs.
- [ ] Intro scan.
- [ ] Track log and external launchers.
- [ ] Playlist: remove rows, reorder. Rows only clear as a whole.
- [ ] Shuffle reorders nothing: it picks random next rows.
- [ ] Compact view.
- [ ] Resume: position saved per folder but not restored on launch.
- [ ] Column sorting in the track list.
- [ ] Cover art pane.
- [ ] Scan-on-launch and folder auto-refresh every 3 s.
- [ ] Keyboard shortcut parity with the Rust build (only the common ones are bound).
- [ ] MKV, WebM, Opus-in-CAF: unsupported by AVFoundation, no library chosen.
- [ ] Settings migration from settings.toml, bookmarks, resume points: not imported.

## Decisions

- Swift package at repo root. AppKit only. No SwiftUI.
- AVFoundation decodes most formats. libvorbisfile for Ogg Vorbis. libopenmpt for trackers. TagLib for tags.
- Playback stops when the folder changes, so list indexes stay valid.
- Settings in JSON under Application Support. TOML not read.
- Deployment target macOS 14. Homebrew libs target 26; the linker warns.
- Bundle runs on the build machine (ad hoc signed, Homebrew dylibs copied and rewritten).
