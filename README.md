# 1by1

A native macOS directory player. A folder is the queue. Pick a folder in the tree,
its tracks fill the list, and playback walks them in order. A playlist collects
tracks from any folder.

Swift, AppKit, AVFoundation. No Rust, no FLTK, no visualizer.

## Build

Needs Command Line Tools (Swift 6) and Homebrew:

```sh
brew install libopenmpt libvorbis libogg taglib
swift build                 # debug build, binary at .build/debug/OneByOne
./scripts/make-app.sh       # release build, dist/OneByOne.app with dylibs bundled
```

The app bundle is ad hoc signed. It runs on the machine that built it.

## Formats

- AVFoundation: MP1/MP2/MP3, AAC/ADTS, M4A/M4B/M4R (AAC, ALAC), FLAC, WAV, AIFF, CAF.
- libvorbisfile: Ogg Vorbis. Ogg Opus plays through AVFoundation.
- libopenmpt: MOD, XM, S3M, IT, MTM and the other tracker formats. Read-only metadata.
- Cue sheets expand into virtual tracks. `.m3u` and `.m3u8` load as lists.
- Not supported: Matroska, WebM, Opus in CAF.

## Use

- File > Open Folder, or drop a folder on the window.
- Double-click a row to play. Space plays or pauses.
- Play > Add to Playlist (Control-Return) queues the selected rows.
- Play > Track Info (Command-I) edits tags. Tracker modules are read-only.
- Play > Go to Track (Command-G). Find in list is in the toolbar.
- Bookmarks menu saves folders and tracks, up to 30.
- File > Start HTTP Remote runs a control page on 127.0.0.1:8080.
  Paths: `/res /prv /nxt /rew /fwd /vdn /vup /dprv /dnxt /home`.

Settings live in `~/Library/Application Support/1by1/settings.json`.

## Layout

```
Sources/OneByOne/
  App/        entry, delegate, menus, bookmarks menu
  Model/      player controller, settings
  Audio/      gapless engine, PCM sources (AVFoundation, Vorbis, OpenMPT)
  Library/    folder scan, cue and m3u, TagLib tags, module metadata
  Remote/     HTTP remote
  UI/         split view, folder tree, track table, track info, big title, settings
  Logic.swift repeat, next/prev, shuffle, natural sort, clock
scripts/make-app.sh   app bundle with Homebrew dylibs
```

## Self checks

```sh
.build/debug/OneByOne --selftest                 # logic checks
.build/debug/OneByOne --scan <folder>            # what the library sees
.build/debug/OneByOne --smoke <file> [<file>...] # headless gapless playback
```
