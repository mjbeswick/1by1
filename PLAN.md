# 1by1 native macOS rewrite

## Goal

Replace the Rust + FLTK app with a native macOS app in Swift and AppKit.
Keep the directory-player behaviour: folder is the queue, one track at a time.
Drop the Milkdrop visualizer and the Windows build. Keep the app small.

## Decisions

1. Location: Swift package at repo root (`Package.swift`, `Sources/OneByOne/`).
   Rust code stays until parity, then gets deleted in one commit.
2. UI: AppKit only. `NSOutlineView` (folder tree), `NSTableView` (track list),
   `NSToolbar`, `NSSlider`, `NSMenu`, `NSWindow` with `NSVisualEffectView` where useful.
   No SwiftUI, no custom drawing except the big title view.
3. Playback: `AVAudioEngine` + `AVAudioPlayerNode`. A `PCMSource` protocol yields
   `AVAudioPCMBuffer` chunks. Gapless = keep scheduling the next source into the same node.
4. Decode:
   - AVFoundation (`AVAudioFile`): MP1/MP2/MP3, AAC/ADTS, M4A/M4B/M4R (AAC, ALAC),
     FLAC, WAV, AIFF, CAF.
   - libvorbisfile (Homebrew): Ogg Vorbis.
   - libopenmpt (Homebrew): all tracker modules.
   - Dropped: Matroska/WebM, Opus-in-CAF. No library in scope covers them cleanly.
5. Tags: TagLib via `taglib_c` (Homebrew). Read and write ID3, MP4, FLAC, Vorbis.
   Module metadata comes from libopenmpt and is read-only.
6. HTTP remote: `Network.framework` `NWListener`. No dependency.
7. Settings: `Codable` JSON in `~/Library/Application Support/1by1/settings.json`.
   Portable mode and TOML dropped. One-time import of old TOML skipped.
8. Native dependencies (Homebrew): `libopenmpt`, `libvorbis`, `libogg`, `taglib`.
   Wired through a `systemLibrary` / `.systemLibrary` module map with pkg-config.
9. Build: `swift build`, then `scripts/make-app.sh` builds `dist/1by1.app`
   (Info.plist, icon, no bundled dylibs if Homebrew libs are linked statically or
   copied; decide at the packaging step).
10. Toolchain: Swift 6.3 with Command Line Tools. No Xcode, so no XCTest runner;
    tests are small executable checks, as in ponytail style.

## Architecture

```
Sources/OneByOne/
  App/          NSApplication entry, AppDelegate, menus, window controller
  Model/        track model, folder scan, natural sort, repeat/shuffle logic
  Library/      folder scan, file-type table, cue/m3u parsing
  Tags/         TagLib wrapper, AVFoundation metadata fallback
  Audio/        AVAudioEngine player, PCMSource protocol, decoders (AVF, Vorbis, OpenMPT)
  UI/           tree, list, toolbar, status bar, settings, track info, find, about
  Remote/       NWListener HTTP control
  Settings/     Codable settings, resume store, bookmarks, history
CSystem/        module map for libopenmpt, vorbisfile, taglib_c
```

Threading: main thread for UI. Audio on AVAudioEngine's render thread; buffers
produced on a dedicated decode queue. HTTP requests hop to main via `DispatchQueue.main`.

## Feature parity checklist

Phase 1: skeleton
- [ ] Package.swift, system library module map, `swift build` green
- [ ] App launches, main window, native menu bar (File, Play, View, Bookmarks, Help)
- [ ] Folder tree (`NSOutlineView`), lazy children, 3 s refresh
- [ ] Track list (`NSTableView`), sortable columns, natural order, full-row select
- [ ] Status bar, title bar shows track name

Phase 2: playback
- [ ] AVFoundation decoder source, AVAudioEngine output
- [ ] Play, stop/resume with stored position, space resumes
- [ ] Seek slider, rewind/forward 5 s, volume, mute
- [ ] Next/prev track, next/prev folder, auto search dirs
- [ ] Gapless queue
- [ ] Ogg Vorbis source (libvorbisfile)
- [ ] Tracker source (libopenmpt), metadata from module

Phase 3: modes and lists
- [ ] Repeat off/all/track, shuffle, play-one, quit after track, quit at end
- [ ] Playlist view (top/bottom/right), enqueue, play from playlist
- [ ] Cue sheets, m3u/m3u8 loading
- [ ] Per-folder resume, history menus, bookmarks (30)

Phase 4: info and settings
- [ ] Track info window with tag read and write (TagLib)
- [ ] Find files, find in list, go to, explore folder
- [ ] Settings window (JSON), themes follow system appearance
- [ ] Compact view, big title view
- [ ] Track log, external launchers

Phase 5: remote and polish
- [ ] HTTP remote (NWListener), same endpoints as before
- [ ] Drag and drop files and folders
- [ ] CLI-equivalent: open file/folder from Finder and `open -a`
- [ ] Packaging script, Info.plist, icon
- [ ] Delete Rust code, Cargo files, build.rs, vendor/projectm4, scripts for Rust
- [ ] README and docs rewritten for macOS

## Dropped

Milkdrop, projectM, GL fullscreen visualizer, Windows support, portable mode,
TOML config, Matroska/WebM, Opus-in-CAF, equalizer, enhancers, mono downmix,
fade/overlap, streaming encoder, systray, global hotkeys, Winamp plugins, BASS.

## Open questions

- Whether the HTTP remote keeps the exact `/res /prv /nxt ...` paths. Default: keep.
- Whether the Rust `dist/` and `vendor/` are removed at parity. Default: yes.
