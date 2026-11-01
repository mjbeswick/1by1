# 1by1-rs: Rust + FLTK recreation of 1by1 Directory Player

## Goal

Recreate the core of 1by1 v2.17 (freeware Windows directory audio player) in Rust with
FLTK. Play tracks one by one from folders. No playlists required. Clean-room: build
from public documentation and static inspection of the shipped artifact. Do not copy
code, icons, strings or language files. Different internal name.

## Evidence (REA)

Target: `~/Downloads/1by1_217_clean.exe`, 374,722 bytes,
SHA-256 `98525102d2e15764b338a6a4ddb66583024299acb905b7021ea4b6fdda5ba984`.

| Step | Tool | Evidence | Result |
|---|---|---|---|
| Outer artifact | `inspect_pe_resources` | `ev_cbebd7d4…0023309` | PE32, machine 0x14c, 7 resources, one icon group (48/32/16), manifest + version. 13 KB stub. |
| Overlay | file offsets | — | 331 KB overlay from 0xAA00: labels `___SFX_LABELS_START_918273___`, then a ZIP with `1by1.exe` + 11 `lan*.ini` + `Readme.htm`. |
| Payload | `open_binary` + `inspect_pe_resources` | `ev_04262106…7eee16` | `1by1.exe`, SHA-256 `52f52db2…73fd64`, PE32 MinGW, 698,382 bytes, 59 resources: 42 dialogs, 8 bitmaps, 5 icons, 1 accelerator table, 1 group icon, version, manifest. |

Static-only. The program was never executed. Observations:

- Imports: COMCTL32, GDI32, KERNEL32, msvcrt, ole32, SHELL32, USER32 only. No
  CreateWindowEx/GetMessage: the main window is a dialog-style shell; the UI is
  owner-drawn and driven by 42 `DIALOGEX` templates.
- Toolbar strings give the exact button order: Menu, Srch, Expl, Intro, App1-3, HTTP,
  Enhancer 2, Enhancer 1, Mono, Play, Stop/Resume, Prev/Next track, Prev/Next dir,
  Repeat, Playlist, Track=next, Skip, Shuffle, Reload, Home, Scan, Main menu,
  Bookmarks, Go to, Find, Explore, Intro scan, Quit next, Big view, Scrobble, View
  log, Open pic, Mute, File info, External 1-3, Track history, Folder history.
- HTTP remote endpoints: `/res /prv /nxt /rew /fwd /vdn /vup /dprv /dnxt /home`,
  `/1by1.mp3`, `/1by1.m3u`.
- Dialog templates give every settings page and control label, including the
  configuration window with a section tree (Playback, Audio, Decoder, Enhancer 1/2,
  Control, List and info, Button bar, Hotkeys, Special, Window style, List style,
  Big display, System, File options, Webserver, Title options, Logging, About).
- `lanDE.ini` (and siblings) expose the string key layout: menus, tooltips, misc,
  dialogs, long help texts, resource strings, config labels.
- Themes in the binary: Current, Dark mode, Anthracite, Paper, Grey, Deep blue, Blue,
  Red. The clone does not copy them: it follows the OS light/dark appearance.

## Scope of this build

Shipped (v0.1):

- Folder tree, lazy expansion, cover art pane, resume per folder. Placeholder
  children keep folders expandable; expanded nodes and the current list refresh
  every three seconds.
- Sortable multi-column track list (name, length, size+duration, type, modified,
  info), natural order, grid, full-row select, month colour option.
- Playback: every container and codec in symphonia 0.5.5 through rodio's
  `symphonia-all`: MP1/MP2/MP3, AAC/ADTS, MP4/M4A/M4B/M4R (AAC, ALAC), FLAC,
  Ogg Vorbis, WAV (PCM, ADPCM), AIFF, CAF (PCM), Matroska/WebM (MKV, MKA, WEBM).
  Opus and ALAC-in-CAF are not supported by this decoder stack.
- Tracker modules through libopenmpt (MOD/XM/S3M/IT/MTM/UMX/MO3/MPTM and more),
  including module metadata: title, author, tracker, date, message, duration.
- Stop/Resume with stored position, next/prev track, next/prev folder, auto search,
  rewind/forward, volume, mute.
- Repeat off/all/track, shuffle, play-one, quit after track, quit at end, gapless.
- Playlist view in three positions, enqueue, play from playlist.
- Cue sheet expansion, m3u/m3u8 loading.
- Track info with ID3/comment editing (lofty), tag-aware titles and info column.
- Settings window (11 pages) persisted as TOML, themes, compact view, big view.
- Logging, history menus, bookmarks, external launchers, HTTP remote control,
  drag and drop, CLI options.

Not shipped: equalizer, enhancers, mono downmix, fade/overlap, streaming encoder,
systray, global hotkeys, Winamp plugins, BASS.

## Architecture

```
src/main.rs        window assembly, callbacks, CLI args, smoke mode
src/app.rs         state, commands, actions, playback flow, tree, persistence
src/player.rs      rodio output, cue windows, gapless queue, finished tracking
src/library.rs     folder scan, formats, tags (lofty), cue/m3u parsing, tag write
src/logic.rs       repeat/next/shuffle, natural sort, clock and date helpers
src/settings.rs    settings struct, TOML load/save, portable mode
src/theme.rs       light and dark palettes, macOS appearance detection
src/icons.rs       vector glyphs for the toolbar
src/http.rs        remote control server (thread + channel)
src/ui/mod.rs      layout, tree theme, cover pane
src/ui/widgets.rs  toolbar, flat sliders, big view
src/ui/track_list.rs  custom-drawn table with columns, selection, scroll
src/ui/dialogs.rs  settings, info, find, goto, about, rename, log
src/ui/menus.rs    main, track, bookmarks, history menus
```

Threading: one FLTK thread. Audio runs in rodio's mixer thread. HTTP clients post
commands over an mpsc channel drained by the 120 ms tick. Menu and dialog callbacks
only enqueue messages; the 30 ms dispatcher runs them, so nested FLTK event loops
never re-enter the app state.

## Decisions

1. Target Windows x86_64 in spirit, developed and smoke-tested on macOS ARM.
   Windows cross-check blocked: no mingw-w64 toolchain on this machine.
2. Audio: rodio 0.23 with symphonia-all. Gapless implemented by appending the next
   source to the same player and advancing when the queue length drops.
3. Tags: lofty 0.25. Info column and track info use it; tag writes go through
   `save_to_path`.
4. Config: TOML, not INI. Portable mode via `/localini` or a `settings.toml` next to
   the binary.
5. Custom-drawn list and toolbar instead of native list views; the original is
   owner-drawn and this keeps colour control.
6. Name: crate `onebyone`, window title "1by1", bundle id placeholder.

## UI pass

- Native macOS menu bar through `SysMenuBar`; `NoWindowMenu` removes the empty
  `Window` menu. Menus: File, Play, View (theme submenu, size toggles), Bookmarks,
  Help.
- Toolbar regrouped to 24 buttons in five visual groups, 28 px targets, vector icons
  redrawn on a 100x100 grid, separators and a two-pixel active underline instead of
  boxes.
- Layout: 8 px outer margin, 36 px toolbar, 16 px slider row, 6 px pane gaps,
  1 px borders around tree, cover, list, playlist and big view, 22 px status strip
  with a hairline above it.
- Table: 20 px header and rows, column dividers, padding, window-aware column widths,
  thin scrollbar, rounded slider knobs.
- Settings pages: wider row spacing, indented section tree.

## Verification

- `cargo test`: 9 tests pass (playback logic, repeat/shuffle, natural sort, dates,
  sizes, settings round-trip, cue time, tag write round-trip, decoder seek fixtures
  when `ONEBYONE_FIXTURES` is set).
- `ONEBY1_SMOKE=1` launches, plays the first track, scans, toggles playlist and big
  view, prints state and quits.
- Manual runs: playback and seeking, resume, repeat/shuffle, playlist enqueue,
  cover art (folder.jpg), dark theme, track info, find dialogs, settings, F1 menu,
  native menu shortcuts, compact view, HTTP `/nxt` returning and changing track,
  logging to `1by1.log`.
- Not verified: Windows build, audible quality by ear, Shift-based shortcuts through
  automation (modifier delivery in the test harness is unreliable).

## Remaining work

- Windows build and shortcut smoke test.
- Slider drag polish; tooltip theme; settings page for hotkeys.
- DSP chain and streaming encoder if full parity is wanted.
