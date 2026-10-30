# 1by1-rs: Rust + FLTK recreation of 1by1 Directory Player

## Goal

Recreate the core of 1by1 v2.17 (freeware Windows directory audio player) in Rust with FLTK.
Play tracks one by one from folders. No playlists needed. Clean-room: build from the
public Readme feature list and observed UI behaviour only. Do not copy the binary, its
code, its icons or its language files. Use a different product name.

## What the inspected binary shows

Source: `~/Downloads/1by1_217.exe`, SHA-256 `e6b586…977d91`. Static only, not run.

- Self-extracting ZIP. Payload: `1by1.exe` (PE32, x86), 12 `lan*.ini` language files,
  `Readme.htm`, `License.txt`.
- Win32 GUI, MinGW-built, per-monitor DPI aware, `asInvoker`. Not Authenticode-signed.
- Audio via BASS (per Readme). Formats: MP3, WAV, OGG, AAC, MP4, FLAC, CD, plus plugins.
- Has an HTTP/streaming server, a systray icon, logging, WAV export, cue sheets, playlists.
- Network strings: a `1by1.mp3` URL fetch, a BBC RSS feed, a Discogs search link.
  Treat the RSS and Discogs items as optional. Do not carry them into MVP.

## Progress

- [x] Cargo project scaffolded, `fltk` dependency added
- [x] Window skeleton builds (transport row, placeholder body, status bar), no wiring
- [ ] Decisions in "Open questions" resolved

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
| Decode | `symphonia` (MP3, FLAC, WAV, OGG, AAC, MP4) | BASS (closed binary, licence) | Pure Rust, no DLL, permissive licence. |
| Output | `cpal` | `rodio` | rodio hides the sink we need for gapless and volume later. Add `rodio` only if cpal gets painful. |
| Tags and duration | `lofty` | `id3` crate | One crate for ID3, Vorbis, MP4 tags. |
| Config | `serde` + `toml` in `dirs` config path | Registry | Portable. Matches "portable" mode of original. |
| Tree walk | `walkdir` | std recursion | Less code. |

Target: Windows x86_64 first. Build with `cargo build --release`.
Cross-compile check on macOS with `x86_64-pc-windows-gnu` later, if useful.

## Architecture (lean)

- `src/main.rs`: window, menus, shortcuts, wiring. One file to start.
- `src/player.rs`: decode thread, output stream, play state, position. Channel to UI.
- `src/library.rs`: folder tree and track list model. Scan on a worker thread.
- `src/settings.rs`: load and save TOML.

Keep UI to player-thread messages (`mpsc`). No shared mutable state across threads.

## Open questions (need your call)

1. Windows only, or macOS too for dev? Plan assumes Windows target, dev on macOS.
2. Audio stack OK with symphonia + cpal, or must it be BASS-equivalent output quality?
3. Name for the product. Plan uses `1by1-rs` as the working name.
4. MVP scope above OK, or cut more (e.g. drop resume play)?
5. Ship a shared FLTK build with static linking, or dynamic? Static is simpler for a single exe.

## Verification

- [ ] Unit check on the position and resume logic (`assert`-based or one small test file)
- [ ] Play a set of mixed-format test files end to end
- [ ] Keyboard shortcuts smoke test on Windows build
- [ ] Compare behaviour against original 1by1 on same folder (manual, one-off)

## Notes

- REA MCP is not registered in this Claude Code session. `rea doctor` reports:
  Node 24.13.1 below required 24.22.1, no REA registration, no Ghidra/Hopper/IDA engines.
  This plan uses static strings, PE headers, and the bundled Readme only.
  To get import and resource tables, run `rea setup` after upgrading Node, or use
  `pefile` (`pip install pefile`) for a quick import dump.
- Open items in this plan are not decided. Resolve before generating code beyond the skeleton.
