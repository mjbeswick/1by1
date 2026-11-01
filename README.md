# 1by1-rs

A clean-room remake of the 1by1 Directory Player (v2.17) in Rust with FLTK.

No library, no database: a folder is the queue. Pick a folder in the tree, the files
appear in the list, and playback walks the list. A second playlist view can collect
tracks from anywhere.

The original was never run for this work. Behaviour comes from the public manual and
from static inspection of the shipped Windows binary: PE resources (42 dialog
templates, 8 bitmaps, the icon group), the language file key layout, and the strings
inside the executable. No code, icons, text or language files from the original are
used. Every glyph is drawn from primitives, every string is written for this build.

## Build

Needs a Rust 1.85+ toolchain and CMake (the `fltk` crate builds FLTK from source).
Tracker modules need libopenmpt. On macOS: `brew install libopenmpt`.

```sh
cargo build --release
./target/release/onebyone [folder|file|playlist]
```

macOS:

```sh
./scripts/make-app.sh          # builds dist/1by1.app
```

On macOS the application menu bar is native (`File`, `Play`, `View`, `Bookmarks`,
`Help`). The empty FLTK `Window` menu is turned off with
`WindowMenuStyle::NoWindowMenu`.

## What works

- Folder tree with lazy expansion, cover art below the tree, per-folder resume.
  The tree and the current list poll every three seconds and pick up added,
  removed or renamed entries without losing the playing track.
- Track list with sortable columns, natural order, month-ish colouring option, grid
  lines, full-row select, extra info column with ID3 fields.
- Playback for MP1/MP2/MP3, AAC/ADTS, MP4/M4A/M4B/M4R (AAC and ALAC), FLAC,
  Ogg Vorbis, WAV (PCM and ADPCM), AIFF, CAF (PCM) and Matroska/WebM audio.
  Opus is not in symphonia 0.5, and ALAC inside CAF is not decoded; PCM CAF is.
- Tracker modules through libopenmpt: MOD, XM, S3M, IT, MTM, UMX, MO3, MPTM, 669,
  FAR, MED, STM, PTM, ULT, OKT, MDZ, DBM, DMF, DSM, AMF, GDM, J2B, MT2, PSM.
  Title, author, tracker, save date, song message and length are read from the
  module and shown in the list, track info and title bar. Module metadata is
  read-only.
- Stop/Resume remembers the file and position, even across restarts. Space resumes.
- Next/previous track, next/previous folder, auto search dirs, rewind/forward.
- Repeat off/all/track, shuffle, "play one track", quit after track, quit at end.
- Gapless output: the next track is queued into the same player before the end.
- Playlist view (top/bottom/right), enqueue with Ctrl+Return, play from it.
- Cue sheets expand into virtual tracks; `.m3u` lists load into the file list.
- Track info window with editable ID3 tags/comment written through lofty.
- Settings window with eleven pages, persisted in TOML.
- Track logging, history menus, bookmarks (30), external launchers, HTTP remote.
- Drag and drop files and folders onto the window.
- Hover hints: toolbar buttons name their action and shortcut in the status strip and
  as a tooltip; sliders and list rows carry tooltips too.
- Toolbar icons are [Lucide](https://lucide.dev) SVGs (ISC licence), embedded and
  rasterised by FLTK at display resolution.

## Shortcuts (selection)

| Key | Action |
|---|---|
| Space | Stop / resume |
| Return | Play selected track |
| Left / Right | Rewind / forward 5 s |
| Ctrl+Left / Right | Previous / next track |
| Shift+Left / Right | Previous / next folder |
| Ctrl+Up / Down | Volume |
| Pause | Mute |
| Ctrl+R | Cycle repeat |
| Ctrl+P | Shuffle on/off |
| Ctrl+B | Big title view |
| F4 | Folder tree on/off |
| F5 | Reload folder |
| F6 | Settings |
| F7 | Compact view |
| Shift+F5 | Playlist view |
| F3 | Find files |
| Ctrl+F | Find in list |
| Ctrl+I / Alt+Return | Track info |
| Ctrl+N | Intro scan |
| Ctrl+Q | Quit |
| Ctrl+Shift+Q | Quit after this track |
| Ctrl+X | Explore folder |
| Del / Ins | Skip / track = next |
| Ctrl+Shift+W | HTTP server |
| Ctrl+Shift+L | Track log |

Command line: `onebyone [file|folder|playlist] [/r /rt /rd /close /hide /enqueue /newinst /localini]`.

## Interface

Flat layout: 8 px window margin, a 36 px button bar with grouped Lucide icons,
thin position and volume sliders, bordered panes with 6 px gaps, and a status strip
with a hairline separator. Colours follow the macOS light or dark appearance and
switch live within two seconds. Compact view shrinks the window to the button bar.

## Config

`$XDG_CONFIG_HOME/1by1/settings.toml`, `~/.config/1by1/settings.toml`, or
`%APPDATA%\1by1\settings.toml`. `/localini` (or an existing `settings.toml` next to
the binary) switches to portable mode. The log file sits beside the settings file.

## Not implemented

DSP: the equalizer, the single-band and multiband enhancers, mono downmix, fade and
overlap mixing. Streaming encoder (`lame_enc`/ACM), systray icon, global hotkeys,
Winamp input plugins, BASS. Local HTTP remote works, MP3 streaming does not.
