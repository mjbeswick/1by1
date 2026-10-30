# 1by1-rs: Rust + FLTK recreation of 1by1 Directory Player

## Goal

Recreate the core of 1by1 v2.17 (freeware Windows directory audio player) in Rust with FLTK.
Play tracks one by one from folders. No playlists needed. Clean-room: build from the
public Readme feature list and observed UI behaviour only. Do not copy the binary, its
code, its icons or its language files. Use a different product name.

## What the inspected binary shows

Source: `~/Downloads/1by1_217_clean.exe`, 374,722 bytes, SHA-256 `98525102…5ba984`.
Static only, not run. Payload not extracted. Earlier notes used a different build
(`e6b586…977d91`). Its details are superseded.

- Self-extracting ZIP. Payload: `1by1.exe`, 11 `lan*.ini` language files, `Readme.htm`.
  No `License.txt` in this build. Version string: 2.17, 2026-08-21.
- The Readme is the only spec read. Readme text was extracted from the installer with
  `zipfile`. Nothing else from the payload was opened.
- Win32 GUI, MinGW-built, per-monitor DPI aware, `asInvoker`. Not Authenticode-signed.
- Audio via BASS (per Readme). Formats: MP3, WAV, OGG, AAC, MP4, FLAC, CD, plus plugins.
- Has an HTTP/streaming server, a systray icon, logging, WAV export, cue sheets, playlists.
- Network strings: a `1by1.mp3` URL fetch, a BBC RSS feed, a Discogs search link.
  Treat the RSS and Discogs items as optional. Do not carry them into MVP.

## Progress

- [x] Cargo project scaffolded, `fltk` dependency added
- [x] Window skeleton builds (transport row, placeholder body, status bar), no wiring
- [x] Decisions in "Open questions" resolved (see below)
- [x] Readme read; MVP shortcut list written below
- [x] Core MVP wired: tree, list, playback, repeat, shuffle, volume, mute, skip, resume, drop
- [ ] Windows build not run

## Scope

**MVP (v0.1)**
- [ ] Folder tree plus track list (FLTK `Tree` + `Table`)
- [ ] Play, pause, stop, next, prev. Click or Enter to start a track.
- [ ] Continue to next track in folder when one ends
- [ ] Resume play: remember file and position on close
- [ ] Repeat modes (off, all, one), shuffle, order cycle
- [ ] Volume (Ctrl+Up/Down), mute (Pause)
- [ ] Keyboard shortcuts from Readme table (subset listed below)
- [ ] Persisted settings (TOML in the user config dir)
- [ ] Drag and drop files or folders onto the window

**Later**
- [ ] File finder (F3), folder finder (Shift+F3)
- [ ] Playlists and cue sheets
- [ ] Intro scan, track times scan
- [ ] Gapless and crossfade
- [ ] Equalizer, multiband enhancer
- [ ] Systray, global hotkeys
- [ ] Bookmarks
- [ ] HTTP server
- [ ] WAV export
- [ ] Languages (start with English only)
- [ ] Drop or skip: RSS, Discogs, BBC lookups

## Technical plan

| Concern | Choice | Alternative rejected | Why |
|---|---|---|---|
| GUI | `fltk` crate (fltk-rs), FLTK 1.4 | egui, native Win32 via `windows` crate | Matches the look and small footprint. Native-feel widgets. |
| Decode + output | `rodio` 0.23, `symphonia-all` (MP3, FLAC, WAV, OGG, AAC, MP4) | `symphonia` + `cpal` by hand | Resampling and channel conversion come free. Hand-rolled was the painful part. Supersedes the earlier choice. |
| Tags | none in MVP | `lofty` | Not needed for filename-only list. Add later. |
| Config | `serde` + `toml`, path from env | `dirs`, Registry | Portable. Fewer deps. |
| Tree walk | std `fs::read_dir` | `walkdir` | Less code. |
| Gapless (later) | rodio mixer, or cpal if rodio blocks it | — | Revisit at Later stage. |

Target: Windows x86_64 first. Build with `cargo build --release`.
Cross-compile check on macOS with `x86_64-pc-windows-gnu` later, if useful.

## Architecture (lean)

- `src/main.rs`: window, menus, shortcuts, wiring. One file to start.
- `src/player.rs`: decode thread, output stream, play state, position. Channel to UI.
- `src/logic.rs`: pure next/prev/repeat/shuffle and decode-skip logic. No fltk, no rodio.
- `src/settings.rs`: load and save TOML. Pure.
- `src/library.rs`: folder scan (sync, `read_dir`) and audio-file filter.

Single thread. All widgets and playback driven from FLTK callbacks and one timer.

## Decisions (defaults taken, not asked)

1. Target Windows x86_64. Dev and smoke-run on macOS. Windows build not yet built.
2. Audio: `rodio` 0.23 with `symphonia-all`. Replaces `symphonia` + `cpal`. Reason: rodio
   handles resampling and channel conversion. Hand-rolled cpal needed both.
3. Name: keep `1by1-rs` and crate `onebyone`. One const `APP_NAME` in `main.rs`. Rename before any distribution.
4. MVP scope as listed. Resume play kept.
5. FLTK default build (fltk-sys). Static vs dynamic decided at packaging.

Other choices made during build:

- Tree: full recursive folder scan on root add. `// ponytail:` marks it. Lazy per-node scan if big libraries stall.
- Track list: `HoldBrowser`, not `Table`. Plain list is enough for MVP.
- No `walkdir`, no `lofty`, no `dirs`. Config path from env vars.
- Settings: TOML at `<config>/1by1-rs/settings.toml`.
- Playback: one `Player` per track. 0.25s FLTK timeout polls position and `empty()`. No player thread.
- Repeat, next/prev, decode-skip cap: pure functions in `src/logic.rs`, one `#[test]`.

## UI pass (clean room, Readme only)

Source: Readme text. The original was not run and no screenshots were taken.

- [x] Status bar: message left, mode flags right (REPEAT, SHUF, VOL, MUTE)
- [x] Title bar shows current track
- [x] F4 toggles tree
- [ ] Sort tabs. Skipped: single-folder list has no path sort yet.
- [ ] F7 compact view, F1 main menu, F6 settings window, context menus, big view

## MVP shortcuts (from Readme, subset)

| Key | Action |
|---|---|
| Space | Stop / resume (resume uses saved file and position) |
| Return | Play selected track |
| Ctrl+Up / Ctrl+Down | Volume |
| Ctrl+Left / Ctrl+Right | Previous / next track |
| Pause | Mute |
| Ctrl+R | Cycle repeat (off, all, one) |
| Ctrl+O | Toggle shuffle (Readme: cycle order type; full cycle later) |
| Ctrl+H | Go home (current track) |
| Del | Skip track (removed from list until reload) |
| F5 or Ctrl+End | Rescan tree and list |
| Ctrl+Q | Quit |

Not in MVP: Ins, Shift+(Pg)Up/Down move, F3 finder, F4 tree toggle, F7 compact, Ctrl+S, Ctrl+N intro scan, and the rest of the Readme list.

## Verification

- [x] Unit check on pure logic and settings round-trip (`cargo test`)
- [x] Seek check per format: MP3, FLAC, WAV, M4A, MP4, Vorbis OGG pass. Opus-in-OGG does not decode (symphonia). Run: `ONEBYONE_FIXTURES=<dir> cargo test`
- [x] Launch smoke test on macOS: window, tree, list, restored resume state. Audible playback not checked.
- [ ] Play a set of mixed-format test files end to end by ear
- [ ] Keyboard shortcuts smoke test on Windows build
- [ ] Compare behaviour against original 1by1 on same folder (manual, one-off)

## Notes

- REA MCP is not registered in this Claude Code session. `rea doctor` reports:
  Node 24.13.1 below required 24.22.1, no REA registration, no Ghidra/Hopper/IDA engines.
  This plan uses static strings, PE headers, and the bundled Readme only.
  To get import and resource tables, run `rea setup` after upgrading Node, or use
  `pefile` (`pip install pefile`) for a quick import dump.
- Open items in this plan are not decided. Resolve before generating code beyond the skeleton.
