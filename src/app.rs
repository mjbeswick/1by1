use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    io::Write,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    time::{Duration, Instant, SystemTime},
};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn md_gl_draw_frame(data: *const u8, w: i32, h: i32, win_w: i32, win_h: i32);
}

use fltk::{
    app,
    enums::{Font, Key, Shortcut},
    image::SharedImage,
    prelude::*,
    tree::{TreeItem, TreeReason},
};

use crate::{
    APP_NAME,
    http::{HttpServer, Remote},
    library::{self, Track},
    logic::{self, natural_cmp, progress, Repeat, SortKey},
    player::{Audio, Spec},
    settings::{self, Bookmark, CoverMode, PlaylistPos, Settings, TitleMode},
    theme::Palette,
    ui::{
        self, dialogs, menus,
        track_list::{ListEvent, ListModel},
        widgets::Cmd,
        SplitKind, Ui,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Files,
    Playlist,
}

/// Messages queued from menu and dialog callbacks, which run inside nested event loops.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Act {
    Settings,
    About,
    Info,
    Find,
    FindFiles,
    Goto,
    Refresh,
    ToggleTree,
    TogglePlaylist,
    ToggleBig,
    ToggleCompact,
    ToggleMute,
    Play,
    Pause,
    PlayAt(usize),
    SetRepeat(Repeat),
    Stop,
    Prev,
    Next,
    PrevDir,
    NextDir,
    Repeat,
    Shuffle,
    OneTrack,
    QuitTrack,
    QuitEnd,
    SortBy(SortKey),
    RemoveSelected,
    MoveSel(i32),
    TrackNext,
    Enqueue,
    SelectAll,
    ContentToPlaylist,
    Explore,
    OpenStandard,
    CopyPath,
    Rename,
    Delete,
    NextPreset,
    Visualizer,
    VisualFull,
    CoverMode(CoverMode),
    BookmarkTrack,
    BookmarkFolder,
    ClearBookmarks,
    LoadBookmark(usize),
    LoadRecentTrack(usize),
    LoadRecentDir(usize),
    IntroScan,
    Scan,
    ViewLog,
    OpenPicture,
    HttpToggle,
    External(usize),
    PlaylistPos(PlaylistPos),
    ToggleAutoSearch,
    ToggleFocused,
    ToggleGapless,
    ToggleCover,
    ToggleEscQuit,
    AddFolderRoot,
    RemoveRoot,
    LoadPath(String),
    SavePlaylist,
    AppendPlaylist,
    Quit,
    ToggleLog,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Msg {
    Act(Act),
    Cmd(Cmd),
}

pub struct Intro {
    pub start: Instant,
    pub per_track: Duration,
}

pub struct App {
    pub s: Settings,
    pub cfg: PathBuf,
    pub pal: Palette,
    pub ui: Ui,
    pub dir: Option<PathBuf>,
    pub audio: Option<Audio>,
    pub cur: usize,
    pub focus: Focus,
    pub queued_next: bool,
    pub quit_after: bool,
    pub quit_at_end: bool,
    pub intro: Option<Intro>,
    pub log_lines: Vec<String>,
    pub last_save: Instant,
    pub pending: Rc<RefCell<VecDeque<Msg>>>,
    pub dirs: Vec<PathBuf>,
    pub status_msg: String,
    pub dirty: bool,
    pub http: Option<HttpServer>,
    pub me: Weak<RefCell<App>>,
    #[allow(dead_code)]
    pub started: Instant,
    pub full_size: (i32, i32),
    pub panel_drag: Option<(SplitKind, i32, i32, i32)>,
    pub milkdrop: Option<crate::milkdrop::Milkdrop>,
    pub milkdrop_failed: bool,
    pub visual_full: Option<(fltk::window::GlWindow, Rc<std::cell::RefCell<(Vec<u8>, i32, i32)>>)>,
    pub hover_hint: Option<String>,
    pub os_dark: bool,
    pub last_theme_check: Instant,
    pub last_fs_check: Instant,
    pub list_sig: u64,
    pub list_from_dir: bool,
}

impl App {
    pub fn panel_press(&mut self, kind: SplitKind, x: i32, y: i32) {
        let base = match kind {
            SplitKind::Tree => self.s.tree_width,
            SplitKind::Cover => self.s.cover_height,
            SplitKind::Playlist => match self.s.playlist_pos {
                PlaylistPos::Right => self.s.playlist_width,
                _ => self.s.playlist_height,
            },
        };
        self.panel_drag = Some((kind, x, y, base));
    }

    pub fn panel_drag_to(&mut self, x: i32, y: i32) {
        let Some((kind, sx, sy, base)) = self.panel_drag else {
            return;
        };
        let (dx, dy) = (x - sx, y - sy);
        let max_tree = (self.ui.win.w() - 200).max(140);
        let max_h = (self.ui.win.h() - 260).max(120);
        let max_w = (self.ui.win.w() - 300).max(180);
        match kind {
            SplitKind::Tree => {
                self.s.tree_width = (base + dx).clamp(140, max_tree);
            }
            SplitKind::Cover => {
                self.s.cover_height = (base - dy).clamp(60, max_h);
            }
            SplitKind::Playlist => match self.s.playlist_pos {
                PlaylistPos::Top => {
                    self.s.playlist_height = (base + dy).clamp(80, max_h);
                }
                PlaylistPos::Bottom => {
                    self.s.playlist_height = (base - dy).clamp(80, max_h);
                }
                PlaylistPos::Right => {
                    self.s.playlist_width = (base - dx).clamp(180, max_w);
                }
            },
        }
        let st = self.s.clone();
        self.ui.relayout(&st);
    }

    pub fn panel_release(&mut self) {
        if self.panel_drag.take().is_some() {
            self.save_soon();
        }
    }

    pub fn files(&self) -> Rc<RefCell<ListModel>> {
        self.ui.files.model.clone()
    }

    pub fn playlist(&self) -> Rc<RefCell<ListModel>> {
        self.ui.playlist.model.clone()
    }

    pub fn track(&self) -> Option<Track> {
        let m = self.ui.files.model.borrow();
        m.tracks.get(self.cur).cloned()
    }

    /// Tracks selected in the file list, in list order.
    pub fn selected_tracks(&self) -> Vec<Track> {
        let rows = self.files().borrow().selected_rows();
        let m = self.files();
        let m = m.borrow();
        rows.iter()
            .filter_map(|r| m.tracks.get(*r).cloned())
            .collect()
    }

    /// The one selected track, or `None` when zero or several are selected.
    pub fn selected_track(&self) -> Option<Track> {
        let tracks = self.selected_tracks();
        (tracks.len() == 1).then(|| tracks[0].clone())
    }

    pub fn duration(&self) -> Duration {
        self.track().and_then(|t| t.duration()).unwrap_or_default()
    }

    pub fn pos(&self) -> Duration {
        self.audio.as_ref().and_then(Audio::pos).unwrap_or_default()
    }

    pub fn say(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
        if self.hover_hint.is_none() {
            self.ui.status.set_label(&self.status_msg);
        }
    }

    /// Toolbar hover text shown in the status strip.
    pub fn set_hover_hint(&mut self, hint: Option<String>) {
        let hint = if self.s.show_tooltips { hint } else { None };
        self.hover_hint = hint;
        let text = self
            .hover_hint
            .clone()
            .unwrap_or_else(|| self.status_msg.clone());
        self.ui.status.set_label(&text);
    }

    #[allow(dead_code)]
    pub fn push(&self, m: Msg) {
        self.pending.borrow_mut().push_back(m);
    }

    // ---- files and folders -------------------------------------------------

    pub fn load_dir(&mut self, dir: PathBuf) {
        self.s.last_dir = Some(dir.clone());
        self.dir = Some(dir.clone());
        self.refresh_dir();
        self.ui.cover.set_text(&display_name(&dir));
        self.ensure_tree_root(&dir);
        self.restore_folder_state(&dir);
        self.select_tree_for(&dir);
        self.ui.files.wid.take_focus().ok();
        self.focus = Focus::Files;
        self.refresh_cover();
        self.update_title();
        self.update_big();
        self.save_soon();
    }

    fn restore_folder_state(&mut self, dir: &Path) {
        if !self.s.remember_positions {
            return;
        }
        let entry = self
            .s
            .folder_state
            .iter()
            .find(|(d, _, _)| d == dir)
            .cloned();
        let Some((_, file, pos)) = entry else { return };
        let idx = {
            let m = self.files();
            let m = m.borrow();
            m.tracks.iter().position(|t| t.path == file)
        };
        if let Some(i) = idx {
            self.cur = i;
            self.s.last_pos = pos;
            let h = self.ui.files.wid.h();
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.cur = Some(i);
            m.focus = i;
            m.sel.clear();
            m.sel.insert(i);
            m.ensure_visible(i, h);
        }
    }

    pub fn refresh_dir(&mut self) {
        let Some(dir) = self.dir.clone() else { return };
        let mut tracks = library::folder_tracks(&dir);
        if self.s.folders_in_list {
            for sub in library::subdirs(&dir) {
                let name = sub
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut t = Track::new(sub);
                t.name = format!("[{name}]");
                t.ext = String::new();
                tracks.insert(0, t);
            }
        }
        let n = tracks.len();
        let kb: u64 = tracks.iter().map(|t| t.size).sum::<u64>() / 1024;
        self.list_sig = tracks_sig(&tracks);
        self.list_from_dir = true;
        self.set_tracks(tracks, true);
        self.say(format!(
            "{n} track{}, {kb} KB",
            if n == 1 { "" } else { "s" }
        ));
        if self.s.always_scan {
            self.scan_all(true);
        }
    }

    pub fn set_tracks(&mut self, mut tracks: Vec<Track>, keep: bool) {
        let keep_path = if keep {
            self.track().map(|t| (t.path.clone(), t.cue))
        } else {
            None
        };
        self.sort_tracks_in(&mut tracks);
        if self.s.shuffle {
            logic::shuffle(&mut tracks, self.seed());
        }
        let new_cur = keep_path
            .and_then(|(p, cue)| tracks.iter().position(|t| t.path == p && t.cue == cue))
            .unwrap_or(0);
        let h = self.ui.files.wid.h();
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.set_tracks(tracks);
            m.sort = self.s.sort;
            m.sort_desc = self.s.sort_desc;
            m.cur = None;
            if !m.tracks.is_empty() {
                m.focus = new_cur;
                m.sel.insert(new_cur);
                m.ensure_visible(new_cur, h);
            }
        }
        self.cur = new_cur;
        self.ui.files.redraw();
    }

    fn seed(&self) -> u64 {
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
    }

    pub fn sort_tracks_in(&self, tracks: &mut [Track]) {
        let desc = self.s.sort_desc;
        let natural = self.s.natural_sort;
        tracks.sort_by(|a, b| {
            let ord = match self.s.sort {
                SortKey::Name => {
                    if natural {
                        natural_cmp(&a.name, &b.name)
                    } else {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                }
                SortKey::Length => a.duration().cmp(&b.duration()),
                SortKey::Size => a.size.cmp(&b.size),
                SortKey::Type => a.ext.cmp(&b.ext),
                SortKey::Modified => a.modified.cmp(&b.modified),
                SortKey::Created => a.created.cmp(&b.created),
            };
            if desc { ord.reverse() } else { ord }
        });
    }

    pub fn sort_by(&mut self, key: SortKey) {
        if self.s.sort == key {
            self.s.sort_desc = !self.s.sort_desc;
        } else {
            self.s.sort = key;
            self.s.sort_desc = false;
        }
        let cur_path = self.track().map(|t| (t.path.clone(), t.cue));
        let mut tracks = std::mem::take(&mut self.files().borrow_mut().tracks);
        self.sort_tracks_in(&mut tracks);
        if self.s.shuffle {
            logic::shuffle(&mut tracks, self.seed());
        }
        let idx = cur_path
            .and_then(|(p, cue)| tracks.iter().position(|t| t.path == p && t.cue == cue))
            .unwrap_or(0);
        let h = self.ui.files.wid.h();
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.tracks = tracks;
            m.sort = self.s.sort;
            m.sort_desc = self.s.sort_desc;
            m.cur = Some(idx);
            m.focus = idx;
            m.sel.clear();
            m.sel.insert(idx);
            m.ensure_visible(idx, h);
        }
        self.cur = idx;
        self.ui.files.redraw();
        self.say(format!(
            "Sorted by {} {}",
            key.label(),
            if self.s.sort_desc { "descending" } else { "ascending" }
        ));
    }

    pub fn scan_all(&mut self, only_missing: bool) {
        let mut tracks = std::mem::take(&mut self.files().borrow_mut().tracks);
        for t in &mut tracks {
            if (only_missing && t.info.is_some()) || t.is_cue() {
                continue;
            }
            t.info = library::scan_info(&t.path);
        }
        self.files().borrow_mut().tracks = tracks;
        self.ui.files.redraw();
        self.update_title();
        self.say("Scan finished.");
    }

    // ---- playback ----------------------------------------------------------

    pub fn play_row(&mut self, row: usize) {
        let len = self.files().borrow().tracks.len();
        if row >= len {
            return;
        }
        self.cur = row;
        self.queued_next = false;
        self.play_current(Duration::ZERO);
    }

    pub fn play_current(&mut self, seek: Duration) {
        let Some(track) = self.track() else { return };
        if track.path.is_dir() {
            self.load_dir(track.path.clone());
            return;
        }
        if library::is_playlist(&track.path) {
            let files = library::read_playlist(&track.path);
            if files.is_empty() {
                self.say("Playlist has no readable entries.");
                return;
            }
            let tracks: Vec<Track> = files.into_iter().map(Track::new).collect();
            self.set_tracks(tracks, false);
            self.cur = 0;
            self.play_current(Duration::ZERO);
            return;
        }
        if track.info.is_none() {
            let info = library::scan_info(&track.path);
            if let Some(t) = self.files().borrow_mut().tracks.get_mut(self.cur) {
                t.info = info;
            }
        }
        let track = self.track().unwrap_or(track);
        let spec = Spec {
            path: track.path.clone(),
            start: track.cue.map(|(s, _)| s).unwrap_or_default(),
            limit: track.cue.and_then(|(s, e)| e.map(|e| e.saturating_sub(s))),
        };
        let vol = self.volume();
        let res = {
            let Some(audio) = self.audio.as_mut() else {
                self.say("No audio output device.");
                return;
            };
            let r = audio.play(spec, seek);
            if r.is_ok() {
                audio.set_volume(vol);
            }
            r
        };
        match res {
            Ok(decoded_len) => {
                self.s.last_file = Some(track.path.clone());
                self.s.last_pos = (track.cue.map(|(s, _)| s).unwrap_or_default() + seek).as_secs_f64();
                self.quit_after = false;
                self.queued_next = false;
                if let Some(d) = decoded_len {
                    self.fill_duration(d);
                }
                self.mark_playing(&track);
                self.log_track(&track);
                self.say(format!("Playing: {}", track.name));
                if self.s.gapless {
                    self.queue_next();
                }
            }
            Err(e) => {
                self.say(format!("Cannot play {}: {e}", track.name));
                self.auto_next();
            }
        }
    }

    /// Fill a missing length from the decoder, for containers without tag support.
    fn fill_duration(&mut self, duration: Duration) {
        let path = self.track().map(|t| t.path);
        let Some(path) = path else { return };
        let cur = self.cur;
        let rc = self.files();
        let mut m = rc.borrow_mut();
        if let Some(t) = m.tracks.get_mut(cur) {
            let decoder = library::decoder_name(&path);
            let info = t.info.get_or_insert_with(|| library::AudioInfo {
                decoder,
                ..Default::default()
            });
            if info.duration.is_none() {
                info.duration = Some(duration);
            }
        }
        drop(m);
        self.ui.files.redraw();
    }

    fn mark_playing(&mut self, track: &Track) {
        if let Some(i) = self
            .files()
            .borrow()
            .tracks
            .iter()
            .position(|t| t.path == track.path && t.cue == track.cue)
        {
            self.cur = i;
        }
        {
            let h = self.ui.files.wid.h();
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.cur = Some(self.cur);
            if !self.s.unselect_on_change {
                m.sel.clear();
                m.sel.insert(self.cur);
            }
            m.focus = self.cur;
            m.ensure_visible(self.cur, h);
        }
        self.update_title();
        self.refresh_cover();
        self.update_big();
        self.refresh_toolbar();
        self.ui.seek.model.borrow_mut().enabled = true;
        self.ui.files.redraw();
    }

    pub fn stop(&mut self) {
        if let Some(a) = self.audio.as_mut() {
            if let Some(p) = a.pos() {
                self.s.last_pos = p.as_secs_f64();
            }
            a.stop();
        }
        self.queued_next = false;
        {
            let mut m = self.ui.seek.model.borrow_mut();
            m.value = 0.0;
            m.enabled = false;
        }
        self.ui.seek.wid.redraw();
        self.say("Stopped. Space resumes.");
        self.refresh_toolbar();
    }

    pub fn is_playing(&self) -> bool {
        self.audio.as_ref().is_some_and(Audio::is_playing)
    }

    pub fn toggle_pause(&mut self) {
        if let Some(a) = &self.audio {
            if a.is_paused() {
                a.resume();
                self.say("Playing.");
                self.refresh_toolbar();
                return;
            }
            if a.is_playing() {
                a.pause();
                self.say("Paused. Space stops.");
                self.refresh_toolbar();
                return;
            }
        }
        self.toggle_stop();
    }

    pub fn toggle_stop(&mut self) {
        if self.is_playing() {
            self.stop();
            return;
        }
        if self.files().borrow().tracks.is_empty() {
            self.say("No track. Pick a folder first.");
            return;
        }
        let track = self.track();
        let resume_same = track
            .as_ref()
            .map(|t| Some(&t.path) == self.s.last_file.as_ref())
            .unwrap_or(false);
        let seek = if resume_same {
            let back = Duration::from_millis(self.s.resume_back_ms as u64);
            Duration::from_secs_f64(self.s.last_pos).saturating_sub(back)
        } else {
            Duration::ZERO
        };
        self.play_current(seek);
    }

    pub fn next_track(&mut self) {
        let len = self.files().borrow().tracks.len();
        if len == 0 {
            return;
        }
        match logic::next_index(len, self.cur, self.s.repeat) {
            Some(i) => {
                self.cur = i;
                self.play_current(Duration::ZERO);
            }
            None => self.end_of_list(),
        }
    }

    pub fn prev_track(&mut self) {
        let len = self.files().borrow().tracks.len();
        if len == 0 {
            return;
        }
        self.cur = logic::prev_index(len, self.cur, self.s.repeat);
        self.play_current(Duration::ZERO);
    }

    fn auto_next(&mut self) {
        if self.quit_after {
            self.save();
            app::quit();
            return;
        }
        if self.s.one_track {
            self.stop();
            self.say("Play one track: stopped.");
            return;
        }
        let len = self.files().borrow().tracks.len();
        if self.s.repeat == Repeat::One && len > 0 {
            self.play_current(Duration::ZERO);
            return;
        }
        if self.s.remove_played && len > 1 {
            let cur = self.cur;
            self.remove_at(&[cur]);
        }
        let len = self.files().borrow().tracks.len();
        let has_next = self.cur + 1 < len;
        if !has_next && self.s.repeat == Repeat::Off {
            self.end_of_list();
            return;
        }
        self.next_track();
    }

    fn end_of_list(&mut self) {
        self.stop();
        if self.s.auto_search_dirs && self.goto_next_dir(true) {
            return;
        }
        if self.s.switch_to_folder_at_end {
            self.say("End of list. Folder list is still selected.");
        } else {
            self.say("End of list.");
        }
        if self.s.reload_at_end {
            self.refresh_dir();
        }
        if self.quit_at_end {
            self.save();
            app::quit();
        }
    }

    pub fn goto_next_dir(&mut self, forward: bool) -> bool {
        if self.dirs.is_empty() {
            return false;
        }
        let Some(cur) = self.dir.clone() else { return false };
        let at = self.dirs.iter().position(|d| *d == cur).unwrap_or(0);
        let len = self.dirs.len();
        for step in 1..=len {
            let idx = if forward {
                (at + step) % len
            } else {
                (at + len - (step % len)) % len
            };
            let d = self.dirs[idx].clone();
            if !library::audio_files(&d).is_empty() {
                self.load_dir(d);
                let first = self.files().borrow().tracks.first().cloned();
                if let Some(t) = first {
                    let i = self.files().borrow().tracks.iter().position(|x| x.path == t.path);
                    if let Some(i) = i {
                        self.play_row(i);
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn seek_to(&mut self, frac: f64) {
        let total = self.duration();
        if total.is_zero() {
            return;
        }
        let pos = Duration::from_secs_f64(total.as_secs_f64() * frac.clamp(0.0, 1.0));
        if let Some(a) = self.audio.as_mut() {
            a.seek(pos);
        }
        let start = self
            .track()
            .and_then(|t| t.cue.map(|(s, _)| s))
            .unwrap_or_default();
        self.s.last_pos = (start + pos).as_secs_f64();
    }

    pub fn rewind(&mut self, forward: bool) {
        let step = Duration::from_secs(if forward {
            self.s.forward_secs as u64
        } else {
            self.s.rewind_secs as u64
        });
        let total = self.duration();
        let pos = if forward {
            (self.pos() + step).min(total)
        } else {
            self.pos().saturating_sub(step)
        };
        if let Some(a) = self.audio.as_mut() {
            a.seek(pos);
        }
        if forward && self.s.next_at_end_of_ffwd && !total.is_zero() && pos >= total {
            self.next_track();
        }
    }

    pub fn volume(&self) -> f32 {
        if self.s.muted { 0.0 } else { self.s.volume }
    }

    pub fn set_volume(&mut self, v: f64) {
        self.s.volume = v.clamp(0.0, 1.0) as f32;
        self.ui.vol.set(self.s.volume as f64);
        let vol = self.volume();
        if let Some(a) = self.audio.as_mut() {
            a.set_volume(vol);
        }
        if self.s.muted && self.s.volume > 0.0 {
            self.s.muted = false;
        }
        self.refresh_flags();
        self.save_soon();
    }

    pub fn volume_step(&mut self, up: bool) {
        let step = if self.s.fine_volume { 0.01 } else { 0.05 };
        let v = (self.s.volume + if up { step } else { -step }).clamp(0.0, 1.0);
        self.set_volume(v as f64);
        self.say(format!("Volume {}%", (v * 100.0).round()));
    }

    pub fn toggle_mute(&mut self) {
        self.s.muted = !self.s.muted;
        let vol = self.volume();
        if let Some(a) = self.audio.as_mut() {
            a.set_volume(vol);
        }
        self.refresh_flags();
        self.refresh_toolbar();
        self.say(if self.s.muted { "Muted" } else { "Unmuted" });
    }

    pub fn cycle_repeat(&mut self) {
        self.s.repeat = self.s.repeat.cycle();
        self.refresh_toolbar();
        let msg = format!("Repeat: {}", self.s.repeat.label());
        self.say(msg);
        self.save_soon();
    }

    pub fn toggle_shuffle(&mut self) {
        self.s.shuffle = !self.s.shuffle;
        let cur = self.track().map(|t| (t.path.clone(), t.cue));
        let mut tracks = std::mem::take(&mut self.files().borrow_mut().tracks);
        if self.s.shuffle {
            logic::shuffle(&mut tracks, self.seed());
        } else {
            self.sort_tracks_in(&mut tracks);
        }
        let idx = cur
            .and_then(|(p, cue)| tracks.iter().position(|t| t.path == p && t.cue == cue))
            .unwrap_or(0);
        let h = self.ui.files.wid.h();
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.tracks = tracks;
            m.cur = Some(idx);
            m.focus = idx;
            m.sel.clear();
            m.sel.insert(idx);
            m.ensure_visible(idx, h);
        }
        self.cur = idx;
        self.ui.files.redraw();
        self.refresh_toolbar();
        self.say(if self.s.shuffle { "Shuffle on" } else { "Shuffle off" });
        self.save_soon();
    }

    // ---- list actions ------------------------------------------------------

    pub fn handle_list(&mut self, which: Focus, ev: ListEvent) {
        match ev {
            ListEvent::Select(_) => {
                self.focus = which;
            }
            ListEvent::Activate(row) => {
                self.focus = which;
                if which == Focus::Files {
                    self.play_row(row);
                } else {
                    self.play_from_playlist(row);
                }
            }
            ListEvent::Context(row, x, y) => {
                self.focus = which;
                menus::track_menu(self, which, row, x, y);
            }
            ListEvent::Header(idx) => {
                if which == Focus::Files {
                    let col = self.files().borrow().cols.get(idx).copied();
                    if let Some(key) = col.and_then(|c| c.key()) {
                        self.sort_by(key);
                    }
                }
            }
            ListEvent::Move(delta) => {
                if which == Focus::Files {
                    self.move_selected(delta);
                }
            }
            ListEvent::SelectAll => {}
        }
    }

    pub fn move_selected(&mut self, delta: i32) {
        let rows = self.files().borrow().selected_rows();
        let Some(&row) = rows.first() else { return };
        let len = self.files().borrow().tracks.len();
        let to = row as i32 + delta;
        if to < 0 || to as usize >= len {
            return;
        }
        let to = to as usize;
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.tracks.swap(row, to);
            m.sel.clear();
            m.sel.insert(to);
            m.focus = to;
        }
        if self.cur == row {
            self.cur = to;
        } else if self.cur == to {
            self.cur = row;
        }
        self.ui.files.redraw();
    }

    fn remove_at(&mut self, rows: &[usize]) {
        let rc = self.files();
        let mut m = rc.borrow_mut();
        let was_current = rows.contains(&self.cur);
        let removed_before = rows.iter().filter(|r| **r < self.cur).count();
        let mut rows = rows.to_vec();
        rows.sort_unstable();
        rows.dedup();
        for r in rows.iter().rev() {
            if *r < m.tracks.len() {
                m.tracks.remove(*r);
            }
        }
        self.cur = self.cur.saturating_sub(removed_before);
        m.sel.clear();
        if m.tracks.is_empty() {
            m.cur = None;
        } else {
            self.cur = self.cur.min(m.tracks.len() - 1);
            m.cur = Some(self.cur);
            m.focus = self.cur;
            m.sel.insert(self.cur);
        }
        let left = m.tracks.len();
        drop(m);
        self.ui.files.redraw();
        if was_current {
            if left == 0 {
                self.stop();
            } else {
                self.play_current(Duration::ZERO);
            }
        }
    }

    pub fn remove_selected(&mut self) {
        let rows = self.files().borrow().selected_rows();
        if rows.is_empty() {
            return;
        }
        let n = rows.len();
        self.remove_at(&rows);
        self.say(format!("{n} track(s) removed from list only."));
    }

    /// Move every selected file to the macOS Trash. Cue entries share one file,
    /// so duplicate paths are removed before anything is touched.
    pub fn delete_selected(&mut self) {
        if self.focus == Focus::Playlist {
            self.say("Delete files from the folder list, not the playlist.");
            return;
        }
        let tracks = self.selected_tracks();
        if tracks.is_empty() {
            self.say("Select a file first.");
            return;
        }
        let mut paths: Vec<PathBuf> = Vec::new();
        for t in &tracks {
            if t.path.is_file() && !paths.contains(&t.path) {
                paths.push(t.path.clone());
            }
        }
        if paths.is_empty() {
            self.say("Nothing to delete.");
            return;
        }
        let n = paths.len();
        let mut names = String::new();
        for p in paths.iter().take(8) {
            if let Some(name) = p.file_name() {
                names.push_str("\n");
                names.push_str(&name.to_string_lossy());
            }
        }
        if n > 8 {
            names.push_str(&format!("\n... and {} more", n - 8));
        }
        let ask = format!(
            "Move {n} file{} to the Trash?{names}",
            if n == 1 { "" } else { "s" }
        );
        if fltk::dialog::choice2_default(&ask, "Move to Trash", "Cancel", "") != Some(0) {
            return;
        }
        let playing = self
            .audio
            .as_ref()
            .filter(|a| a.is_playing())
            .and_then(|_| self.track())
            .map(|t| t.path);
        let mut done = 0usize;
        let mut failed = Vec::new();
        for p in &paths {
            match trash::delete(p) {
                Ok(()) => done += 1,
                Err(e) => failed.push(format!("{}: {e}", p.display())),
            }
        }
        if let Some(p) = playing {
            if paths.contains(&p) {
                self.stop();
            }
        }
        self.refresh_dir();
        if failed.is_empty() {
            self.say(format!(
                "{done} file{} moved to Trash.",
                if done == 1 { "" } else { "s" }
            ));
        } else {
            self.say(format!(
                "{done} moved, {} failed: {}",
                failed.len(),
                failed.first().cloned().unwrap_or_default()
            ));
        }
        self.save_soon();
    }

    pub fn track_next(&mut self) {
        let rows = self.files().borrow().selected_rows();
        let Some(&row) = rows.first() else { return };
        let rc = self.files();
        let mut m = rc.borrow_mut();
        if row == self.cur || row >= m.tracks.len() {
            return;
        }
        let t = m.tracks.remove(row);
        let at = (self.cur + 1).min(m.tracks.len());
        m.tracks.insert(at, t);
        m.sel.clear();
        m.sel.insert(at);
        m.focus = at;
        if row < self.cur {
            self.cur -= 1;
        }
        drop(m);
        self.ui.files.redraw();
        self.say("Track moved after the current one.");
    }

    pub fn enqueue_selected(&mut self) {
        let rows = self.files().borrow().selected_rows();
        let tracks: Vec<Track> = {
            let m = self.files();
            let m = m.borrow();
            rows.iter().filter_map(|r| m.tracks.get(*r).cloned()).collect()
        };
        if tracks.is_empty() {
            return;
        }
        let n = tracks.len();
        {
            let rc = self.playlist();
            let mut p = rc.borrow_mut();
            p.tracks.extend(tracks);
            p.sel.clear();
        }
        if !self.s.playlist_open {
            self.s.playlist_open = true;
            self.ui.relayout(&self.s);
        }
        self.ui.playlist.redraw();
        self.refresh_toolbar();
        self.save_soon();
        self.say(format!("{n} track(s) added to the playlist."));
    }

    pub fn content_to_playlist(&mut self) {
        let tracks = self.files().borrow().tracks.clone();
        if tracks.is_empty() {
            return;
        }
        let n = tracks.len();
        {
            let rc = self.playlist();
            let mut p = rc.borrow_mut();
            p.tracks.extend(tracks);
            p.sel.clear();
        }
        if !self.s.playlist_open {
            self.s.playlist_open = true;
            self.ui.relayout(&self.s);
        }
        self.ui.playlist.redraw();
        self.refresh_toolbar();
        self.save_soon();
        self.say(format!("Content added to playlist ({n} tracks)."));
    }

    pub fn play_from_playlist(&mut self, row: usize) {
        let t = self.playlist().borrow().tracks.get(row).cloned();
        let Some(t) = t else { return };
        let len = self.files().borrow().tracks.len();
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.tracks.push(t);
            m.cur = Some(len);
            m.focus = len;
            m.sel.clear();
            m.sel.insert(len);
        }
        self.cur = len;
        self.play_current(Duration::ZERO);
    }

    // ---- theme, titles, flags ---------------------------------------------

    pub fn apply_list_style_public(&mut self) {
        self.apply_list_style();
    }

    pub fn apply_settings(&mut self) {
        if (self.s.http_enabled && self.http.is_none())
            || (!self.s.http_enabled && self.http.is_some())
        {
            self.toggle_http();
        }
        self.set_theme();
        self.apply_list_style();
        self.ui.relayout(&self.s);
        self.ui.seek.wid.redraw();
        self.ui.vol.set(self.s.volume as f64);
        self.refresh_toolbar();
        self.update_title();
        self.update_big();
        self.refresh_cover();
        self.save_soon();
    }

    fn apply_list_style(&mut self) {
        for list in [self.ui.files.clone(), self.ui.playlist.clone()] {
            let mut m = list.model.borrow_mut();
            m.grid = self.s.list_grid;
            m.full_row = self.s.full_row;
            m.month_colors = false;
            m.scrollbar = !self.s.hide_scrollbars;
            m.rowh = 18;
            m.header_h = 18;
        }
        // File list columns depend on the info option.
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.cols = if self.s.extra_info_col {
                vec![
                    ui::track_list::Col::Name,
                    ui::track_list::Col::Length,
                    ui::track_list::Col::Size,
                    ui::track_list::Col::Type,
                    ui::track_list::Col::Modified,
                    ui::track_list::Col::Info,
                ]
            } else {
                vec![
                    ui::track_list::Col::Name,
                    ui::track_list::Col::Length,
                    ui::track_list::Col::Size,
                    ui::track_list::Col::Type,
                    ui::track_list::Col::Modified,
                ]
            };
        }
        {
            let rc = self.playlist();
            let mut m = rc.borrow_mut();
            m.cols = vec![
                ui::track_list::Col::Name,
                ui::track_list::Col::Length,
                ui::track_list::Col::Type,
            ];
        }
        self.ui.files.set_theme(self.pal);
        self.ui.playlist.set_theme(self.pal);
        self.ui.files.redraw();
        self.ui.playlist.redraw();
    }

    pub fn set_theme(&mut self) {
        self.pal = Palette::os(self.os_dark);
        let pal = self.pal;
        let (r, g, b) = pal.window.to_rgb();
        fltk::app::background(r, g, b);
        fltk::app::background2(r, g, b);
        self.ui.win.set_color(pal.window);
        self.ui.bar.set_theme(pal);
        self.ui.seek.set_theme(pal);
        self.ui.vol.set_theme(pal);
        self.ui.big.set_theme(pal);
        {
            let mut cp = self.ui.cover.pal.borrow_mut();
            *cp = pal;
        }
        self.ui.cover.wid.redraw();
        ui::theme_tree(&mut self.ui.tree, pal, &self.s);
        if std::env::var_os("ONEBY1_DEBUG_THEME").is_some() {
        }
        self.ui.tree_back.set_color(pal.grid);
        self.ui.status_line.set_color(pal.grid);
        self.ui.status.set_color(pal.window);
        self.ui.flags.set_color(pal.window);
        self.ui.status.set_color(pal.bar);
        self.ui.status.set_label_color(pal.text);
        self.ui.flags.set_color(pal.bar);
        self.ui.flags.set_label_color(pal.dim);
        self.ui.win.redraw();
    }

    pub fn update_title(&mut self) {
        let name = self
            .track()
            .map(|t| {
                if self.s.tag_titles {
                    t.artist_title()
                } else {
                    t.name.clone()
                }
            })
            .unwrap_or_else(|| APP_NAME.to_string());
        let resume = self
            .track()
            .map(|t| Some(&t.path) == self.s.last_file.as_ref())
            .unwrap_or(false);
        if self.track().is_none() || (!self.is_playing() && !resume) {
            self.ui.win.set_label(APP_NAME);
            return;
        }
        let pos = self.pos();
        let total = self.duration();
        let times = match self.s.title_mode {
            TitleMode::Name => String::new(),
            TitleMode::NameElapsed => {
                format!(" [{}]", logic::clock_marked(pos, self.s.colon_minutes))
            }
            TitleMode::NameRemaining => format!(
                " [{}]",
                logic::clock_marked(total.saturating_sub(pos), self.s.colon_minutes)
            ),
            TitleMode::NameElapsedTotal => format!(
                " [{} / {}]",
                logic::clock_marked(pos, self.s.colon_minutes),
                logic::clock_marked(total, self.s.colon_minutes)
            ),
        };
        let upcoming = if self.s.upcoming_in_title {
            let next = {
                let m = self.files();
                let m = m.borrow();
                m.tracks.get(self.cur + 1).map(|t| t.name.clone())
            };
            next.map(|n| format!("  > {n}")).unwrap_or_default()
        } else {
            String::new()
        };
        self.ui.win.set_label(&format!("{name}{times}{upcoming} - {APP_NAME}"));
    }

    pub fn refresh_flags(&mut self) {
        let mut p = Vec::new();
        if self.s.repeat != Repeat::Off {
            p.push(format!("REPEAT {}", self.s.repeat.label().to_uppercase()));
        }
        if self.s.shuffle {
            p.push("SHUF".into());
        }
        if self.s.muted {
            p.push("MUTE".into());
        }
        if self.quit_after {
            p.push("QUIT!".into());
        }
        if self.s.log_enabled {
            p.push("LOG".into());
        }
        if self.s.http_enabled {
            p.push("HTTP".into());
        }
        if self.s.continue_focused {
            p.push("FCS".into());
        }
        if self.s.auto_search_dirs {
            p.push("SRCH".into());
        }
        if self.s.gapless {
            p.push("GAPL".into());
        }
        if self.s.auto_space_ms > 0 {
            p.push("SPACE".into());
        }
        p.push(format!("{}%", (self.s.volume * 100.0).round()));
        self.ui.flags.set_label(&p.join("  "));
    }

    pub fn refresh_toolbar(&mut self) {
        {
            let mut m = self.ui.bar.model.borrow_mut();
            m.set_active(Cmd::Repeat, self.s.repeat != Repeat::Off);
            m.set_active(Cmd::Shuffle, self.s.shuffle);
            m.set_active(Cmd::Tree, self.s.show_tree);
            m.set_active(Cmd::Playlist, self.s.playlist_open);
            m.set_active(Cmd::Mute, self.s.muted);
            m.set_active(Cmd::BigView, self.s.big_view);
            m.set_active(Cmd::Http, self.s.http_enabled);
            m.set_active(Cmd::QuitNext, self.quit_after);
            m.set_active(
                Cmd::Pause,
                self.audio.as_ref().is_some_and(Audio::is_paused),
            );
        }
        self.ui.bar.redraw();
        self.refresh_flags();
    }

    pub fn update_big(&mut self) {
        if !self.s.big_view {
            return;
        }
        let t = self.track();
        let title = t
            .as_ref()
            .map(|t| {
                if self.s.tag_titles {
                    t.artist_title()
                } else {
                    t.name.clone()
                }
            })
            .unwrap_or_else(|| "No track".into());
        let sub = t
            .as_ref()
            .and_then(|t| t.info.as_ref().map(|i| i.summary()))
            .unwrap_or_default();
        let info = t
            .as_ref()
            .and_then(|t| t.info.as_ref().and_then(|i| i.tags.album.clone()))
            .unwrap_or_default();
        let time = format!(
            "{} / {}",
            logic::clock_marked(self.pos(), self.s.colon_minutes),
            logic::clock_marked(self.duration(), self.s.colon_minutes)
        );
        let lines = match self.s.big_layout {
            crate::settings::BigLayout::TitleTimes => 2,
            _ => 3,
        };
        {
            let mut m = self.ui.big.model.borrow_mut();
            m.title = title;
            m.sub = sub;
            m.info = info;
            m.time = time;
            m.lines = lines;
            m.padding = self.s.big_padding as i32;
        }
        self.ui.big.redraw();
    }

    // ---- cover art ---------------------------------------------------------

    pub fn next_preset(&mut self) {
        if let Some(m) = self.milkdrop.as_mut() {
            m.next_preset();
            self.say("Next preset.");
        }
    }

    /// Create the Milkdrop engine even while its panel is hidden.
    pub fn ensure_milkdrop(&mut self) -> bool {
        if self.milkdrop.is_none() && !self.milkdrop_failed {
            let w = self.ui.cover.wid.w().max(64);
            let h = self.ui.cover.wid.h().max(64);
            match crate::milkdrop::Milkdrop::new(w, h) {
                Some(m) => self.milkdrop = Some(m),
                None => self.milkdrop_failed = true,
            }
        }
        self.milkdrop.is_some()
    }

    pub fn preset_names(&self) -> Vec<String> {
        self.milkdrop
            .as_ref()
            .map(|m| m.preset_names())
            .unwrap_or_default()
    }

    pub fn show_preset(&mut self, index: u32) {
        if !self.ensure_milkdrop() {
            self.say("Milkdrop presets not found.");
            return;
        }
        let name = self
            .milkdrop
            .as_ref()
            .map(|m| m.preset_name(index))
            .unwrap_or_default();
        if let Some(m) = self.milkdrop.as_mut() {
            m.select_preset(index);
        }
        self.say(format!("Preset: {name}"));
        self.ui.cover.wid.redraw();
    }

    pub fn refresh_cover(&mut self) {
        let st = self.s.clone();
        let mode = if st.cover_art {
            st.cover_mode
        } else {
            CoverMode::Off
        };
        self.ui.cover.set_mode(mode);
        if !st.show_tree || mode == CoverMode::Off {
            self.ui.cover.set_image(None);
            self.ui.set_cover_visible(&st, false);
            return;
        }
        if mode == CoverMode::Milkdrop {
            self.ui.cover.set_image(None);
            self.ui.cover.set_text("");
            self.ui.set_cover_visible(&st, true);
            if self.milkdrop_failed {
                self.ui.cover.set_text("Milkdrop presets not found");
            }
            return;
        }
        let dir = self.dir.clone();
        let names: Vec<String> = st
            .cover_names
            .split(';')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let stem = self
            .track()
            .and_then(|t| t.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
            .unwrap_or_default();
        let mut found = None;
        if let Some(dir) = dir {
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    let Some(name) = p.file_name().map(|n| n.to_string_lossy().to_lowercase()) else {
                        continue;
                    };
                    let img = matches!(
                        library::extension(&p).as_str(),
                        "jpg" | "jpeg" | "png" | "bmp" | "gif"
                    );
                    if !img {
                        continue;
                    }
                    if !stem.is_empty() && name.starts_with(&stem) {
                        found = Some(p);
                        break;
                    }
                    if names.iter().any(|n| *n == name) {
                        found = Some(p);
                    }
                }
            }
        }
        match found.and_then(|p| SharedImage::load(p.to_string_lossy().as_ref()).ok()) {
            Some(img) => {
                self.ui.cover.set_image(Some(img));
                self.ui.cover.set_text("");
                self.ui.set_cover_visible(&st, true);
            }
            None if st.cover_hide_empty => {
                self.ui.cover.set_image(None);
                self.ui.set_cover_visible(&st, false);
            }
            None => {
                self.ui.cover.set_image(None);
                let txt = self
                    .dir
                    .as_ref()
                    .map(|d| display_name(d))
                    .unwrap_or_default();
                self.ui.cover.set_text(&format!("{txt}\nno cover image"));
                self.ui.set_cover_visible(&st, true);
            }
        }
    }

    /// Advance the Milkdrop renderer in the bottom left panel.
    /// Open the Milkdrop panel in a borderless fullscreen window.
    pub fn open_visual_fullscreen(&mut self) {
        if self.visual_full.is_some() {
            return;
        }
        if self.s.cover_mode != CoverMode::Milkdrop {
            self.say("Milkdrop is not the current panel mode.");
            return;
        }
        if !self.ensure_milkdrop() {
            return;
        }
        let (sw, sh) = app::screen_size();
        let (sw, sh) = (sw.max(320.0) as i32, sh.max(240.0) as i32);
        let mut win = fltk::window::GlWindow::default()
            .with_size(sw, sh)
            .with_label("Milkdrop");
        win.set_mode(
            fltk::enums::Mode::Rgb | fltk::enums::Mode::Double | fltk::enums::Mode::Depth,
        );
        win.set_border(false);
        let frame: Rc<std::cell::RefCell<(Vec<u8>, i32, i32)>> =
            Rc::new(std::cell::RefCell::new((Vec::new(), 0, 0)));
        let fr = frame.clone();
        win.draw(move |w| {
            let f = fr.borrow();
            unsafe {
                md_gl_draw_frame(f.0.as_ptr(), f.1, f.2, w.pixel_w().max(1), w.pixel_h().max(1));
            }
        });

        let wa = self.me.clone();
        win.handle(move |_, ev| {
            if ev == fltk::enums::Event::KeyDown
                && fltk::app::event_key() == Key::Escape
            {
                if let Some(a) = wa.upgrade() {
                    if let Ok(mut a) = a.try_borrow_mut() {
                        a.close_visual_fullscreen();
                        return true;
                    }
                }
            }
            false
        });
        win.fullscreen(true);
        win.show();
        win.resize(0, 0, sw, sh);
        let _ = win.take_focus();
        self.visual_full = Some((win, frame));
        self.tick_visualizer();
    }

    pub fn close_visual_fullscreen(&mut self) {
        if let Some((mut win, _)) = self.visual_full.take() {
            win.hide();
        }
    }

    pub fn toggle_visual_fullscreen(&mut self) {
        if self.visual_full.is_some() {
            self.close_visual_fullscreen();
        } else {
            self.open_visual_fullscreen();
        }
    }

    pub fn tick_visualizer(&mut self) {
        if !self.s.cover_art
            || self.s.cover_mode != CoverMode::Milkdrop
            || self.milkdrop_failed
        {
            return;
        }
        let full = self.visual_full.is_some();
        if !self.s.show_tree && !full {
            return;
        }
        let (dw, dh) = if let Some((win, _)) = self.visual_full.as_ref() {
            (win.w(), win.h())
        } else {
            (self.ui.cover.wid.w(), self.ui.cover.wid.h())
        };
        if dw < 20 || dh < 20 {
            return;
        }
        let (iw, ih) = if full && dw > 480 {
            (480, (dh as i64 * 480 / dw as i64).max(20) as i32)
        } else {
            (dw, dh)
        };
        if self.milkdrop.is_none() {
            match crate::milkdrop::Milkdrop::new(iw, ih) {
                Some(m) => self.milkdrop = Some(m),
                None => {
                    self.milkdrop_failed = true;
                    self.ui.cover.set_text("Milkdrop presets not found");
                    self.ui.cover.wid.redraw();
                    return;
                }
            }
        }
        let mono = crate::player::take_visualizer_pcm(8192);
        if full {
            let frame = if let Some(m) = self.milkdrop.as_mut() {
                m.resize(iw, ih);
                m.feed(&mono);
                m.frame_rgba()
            } else {
                None
            };
            if let (Some((px, fw, fh)), Some((win, f))) =
                (frame, self.visual_full.as_mut())
            {
                *f.borrow_mut() = (px, fw, fh);
                win.redraw();
            }
        } else if let Some(m) = self.milkdrop.as_mut() {
            m.resize(iw, ih);
            m.feed(&mono);
            if let Some(img) = m.render() {
                self.ui.cover.set_image(Some(img));
            }
            self.ui.cover.wid.redraw();
        }
    }

    // ---- logging -----------------------------------------------------------

    fn log_track(&mut self, t: &Track) {
        if !self.s.log_enabled {
            return;
        }
        let line = format!(
            "{}  {}\t{}",
            logic::datetime_text(SystemTime::now()),
            t.artist_title(),
            t.path.display()
        );
        self.log_lines.push(line.clone());
        let path = settings::log_path(&self.cfg);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{line}");
        }
        let max = self.s.log_max_kb as u64 * 1024;
        if max > 0 {
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > max {
                    let _ = std::fs::write(&path, "");
                }
            }
        }
        let p = t.path.clone();
        self.push_recent(p, true);
        if let Some(dir) = self.dir.clone() {
            self.push_recent(dir, false);
        }
    }

    fn push_recent(&mut self, path: PathBuf, track: bool) {
        let (list, max) = if track {
            (&mut self.s.recent_tracks, self.s.history_entries)
        } else {
            (&mut self.s.recent_dirs, self.s.history_entries)
        };
        list.retain(|p| *p != path);
        list.insert(0, path);
        list.truncate(max.clamp(1, 30) as usize);
    }

    // ---- persistence -------------------------------------------------------

    pub fn save(&mut self) {
        if self.s.remember_positions {
            if let (Some(dir), Some(t)) = (self.dir.clone(), self.track()) {
                let pos = self.pos().as_secs_f64();
                self.s.folder_state.retain(|(d, _, _)| *d != dir);
                self.s.folder_state.insert(0, (dir, t.path.clone(), pos));
                self.s.folder_state.truncate(10);
            }
        }
        let pl: Vec<PathBuf> = self
            .playlist()
            .borrow()
            .tracks
            .iter()
            .map(|t| t.path.clone())
            .collect();
        self.s.last_playlist = pl;
        self.s.win_size = (self.ui.win.w(), self.ui.win.h());
        self.s.volume = self.s.volume.clamp(0.0, 1.0);
        if let Err(e) = settings::save(&self.cfg, &self.s) {
            self.say(format!("Settings not saved: {e}"));
        }
        self.last_save = Instant::now();
        self.dirty = false;
    }

    pub fn save_soon(&mut self) {
        self.dirty = true;
    }

    // ---- periodic work -----------------------------------------------------

    pub fn tick(&mut self) {
        if let Some(server) = &self.http {
            for cmd in server.poll() {
                match cmd {
                    Remote::Resume => self.toggle_stop(),
                    Remote::Prev => self.prev_track(),
                    Remote::Next => self.next_track(),
                    Remote::Rewind => self.rewind(false),
                    Remote::Forward => self.rewind(true),
                    Remote::VolumeDown => self.volume_step(false),
                    Remote::VolumeUp => self.volume_step(true),
                    Remote::DirBack => {
                        self.goto_next_dir(false);
                    }
                    Remote::DirNext => {
                        self.goto_next_dir(true);
                    }
                    Remote::Home => self.go_home(),
                    Remote::Stop => self.stop(),
                }
            }
        }

        let advanced = self.audio.as_mut().map(|a| a.take_finished()).unwrap_or(0);
        if advanced > 0 {
            if self.queued_next {
                self.queued_next = false;
                self.cur += 1;
                if let Some(t) = self.track() {
                    self.s.last_file = Some(t.path.clone());
                    self.mark_playing(&t);
                    self.log_track(&t);
                }
                if self.s.gapless {
                    self.queue_next();
                }
            } else {
                self.auto_next();
            }
        }

        let playing = self.is_playing();
        if playing {
            let pos = self.pos();
            let total = self.duration();
            self.s.last_pos = pos.as_secs_f64();
            {
                let mut sm = self.ui.seek.model.borrow_mut();
                sm.value = progress(pos, total);
                sm.enabled = !total.is_zero();
            }
            self.ui.seek.wid.redraw();
            let name = self.track().map(|t| t.name).unwrap_or_default();
            let status = format!(
                "{}  [{} / {}]",
                name,
                logic::clock_marked(pos, self.s.colon_minutes),
                logic::clock_marked(total, self.s.colon_minutes)
            );
            if self.hover_hint.is_none() && self.ui.status.label() != status {
                self.ui.status.set_label(&status);
            }
            if self.s.gapless && !self.queued_next && !total.is_zero() {
                if total.saturating_sub(pos) < Duration::from_millis(1500) {
                    self.queue_next();
                }
            }
            self.update_big();
            self.update_title();
        }

        if let Some(intro) = &self.intro {
            if intro.start.elapsed() > intro.per_track {
                self.intro = Some(Intro {
                    start: Instant::now(),
                    per_track: intro.per_track,
                });
                let len = self.files().borrow().tracks.len();
                if len > 0 {
                    self.cur = (self.cur + 1) % len;
                    self.play_current(Duration::ZERO);
                }
            }
        }

        if self.last_fs_check.elapsed() > Duration::from_secs(3) {
            self.last_fs_check = Instant::now();
            self.check_filesystem();
        }

        if self.last_theme_check.elapsed() > Duration::from_secs(2) {
            self.last_theme_check = Instant::now();
            let dark = crate::theme::os_dark();
            if dark != self.os_dark {
                self.os_dark = dark;
                self.apply_settings();
            }
        }

        if self.s.autosave_5min && self.last_save.elapsed() > Duration::from_secs(300) {
            self.save();
        }
        if self.dirty && self.last_save.elapsed() > Duration::from_secs(3) {
            self.save();
        }
    }

    /// Poll the current folder and the expanded tree nodes for changes.
    fn check_filesystem(&mut self) {
        if self.list_from_dir {
            if let Some(dir) = self.dir.clone() {
                let tracks = library::folder_tracks(&dir);
                let sig = tracks_sig(&tracks);
                if sig != self.list_sig {
                    self.list_sig = sig;
                    self.set_tracks(tracks, true);
                    self.say("Folder changed, list updated.");
                }
            }
        }
        self.check_tree();
    }

    /// Read the children of every expanded node and rebuild only changed nodes.
    fn check_tree(&mut self) {
        if !self.s.lazy_tree {
            return;
        }
        let keys: Vec<(String, PathBuf)> = self
            .ui
            .paths
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut changed = Vec::new();
        for (key, dir) in &keys {
            if !self.ui.tree.is_open(key) {
                continue;
            }
            let Some(item) = self.ui.tree.find_item(key) else { continue };
            if item.children() == 1
                && item
                    .child(0)
                    .and_then(|c| c.label())
                    .map(|l| l == PLACEHOLDER)
                    .unwrap_or(false)
            {
                continue;
            }
            let now: Vec<String> = library::child_dirs(dir)
                .into_iter()
                .map(|(n, _)| n)
                .collect();
            let have: Vec<String> = (0..item.children())
                .filter_map(|i| item.child(i))
                .filter_map(|c| c.label())
                .filter(|l| l != PLACEHOLDER)
                .collect();
            if now != have {
                changed.push((key.clone(), dir.clone()));
            }
        }
        for (key, dir) in changed {
            self.rebuild_tree_node(&key, &dir);
        }
    }

    fn rebuild_tree_node(&mut self, key: &str, dir: &Path) {
        let mut tree = self.ui.tree.clone();
        let Some(item) = tree.find_item(key) else { return };
        let _ = tree.clear_children(&item);
        let prefix = format!("{key}/");
        self.ui
            .paths
            .retain(|k, _| k != key && !k.starts_with(&prefix));
        self.ui.paths.insert(key.to_string(), dir.to_path_buf());
        for (name, sub) in library::child_dirs(dir) {
            let has_sub = !library::child_dirs(&sub).is_empty();
            if let Some(mut child) = tree.add_to_parent(&item, &name) {
                if has_sub {
                    let _ = tree.add_to_parent(&child, PLACEHOLDER);
                }
                child.close();
            }
            self.ui.paths.insert(format!("{key}/{name}"), sub);
        }
        let _ = tree.open(key, false);
        self.refresh_dirs();
        if let Some(cur) = self.dir.clone() {
            self.select_tree_for(&cur);
        }
    }

    fn queue_next(&mut self) {
        let next = {
            let m = self.files();
            let m = m.borrow();
            m.tracks.get(self.cur + 1).cloned()
        };
        let Some(t) = next else { return };
        let spec = Spec {
            path: t.path.clone(),
            start: t.cue.map(|(s, _)| s).unwrap_or_default(),
            limit: t.cue.and_then(|(s, e)| e.map(|e| e.saturating_sub(s))),
        };
        let ok = self
            .audio
            .as_mut()
            .map(|a| a.enqueue(spec).is_ok())
            .unwrap_or(false);
        if ok {
            self.queued_next = true;
        }
    }

    // ---- tree --------------------------------------------------------------

    pub fn rebuild_tree(&mut self) {
        let roots = self.s.roots.clone();
        self.ui.tree.clear();
        self.ui.paths.clear();
        for root in roots {
            if !root.is_dir() {
                continue;
            }
            let key = display_name(&root);
            let key = unique_key(&self.ui.paths, key);
            self.add_tree_node(None, key, &root, 1);
        }
        self.refresh_dirs();
        let select = self.dir.clone();
        if let Some(dir) = select {
            let key = self
                .ui
                .paths
                .iter()
                .find(|(_, p)| **p == dir)
                .map(|(k, _)| k.clone());
            if let Some(key) = key {
                let _ = self.ui.tree.select(&key, false);
                let _ = self.ui.tree.open(&key, false);
            }
        }
    }

    fn add_tree_node(&mut self, parent: Option<TreeItem>, key: String, dir: &Path, depth: usize) {
        let mut tree = self.ui.tree.clone();
        let item = match &parent {
            Some(p) => tree.add_to_parent(p, &key),
            None => tree.add(&key),
        };
        let Some(mut item) = item else { return };
        if parent.is_none() {
            item.set_label_font(Font::HelveticaBold);
        }
        self.ui.paths.insert(key.clone(), dir.to_path_buf());
        if self.s.lazy_tree && depth <= 1 {
            for (name, sub) in library::child_dirs(dir) {
                let child_key = format!("{key}/{name}");
                let has_sub = !library::child_dirs(&sub).is_empty();
                if let Some(mut child) = tree.add_to_parent(&item, &name) {
                    // Only folders that really contain something get a lazy marker,
                    // otherwise leaf folders would show an empty child row.
                    if has_sub {
                        let _ = tree.add_to_parent(&child, PLACEHOLDER);
                    }
                    // FLTK items start open; collapse so the marker stays hidden.
                    child.close();
                }
                self.ui.paths.insert(child_key, sub);
            }
            return;
        }
        for (name, sub) in library::child_dirs(dir) {
            let before = item.children();
            self.add_tree_node(Some(item.clone()), name, &sub, depth.saturating_sub(1));
            if item.children() > before {
                if let Some(mut child) = item.child(item.children() - 1) {
                    child.close();
                }
            }
        }
    }

    pub fn expand_node(&mut self, item: &TreeItem) {
        let Ok(key) = self.ui.tree.item_pathname(item) else { return };
        let mut tree = self.ui.tree.clone();
        let placeholder_only = item.children() == 1
            && item
                .child(0)
                .and_then(|c| c.label())
                .map(|l| l == PLACEHOLDER)
                .unwrap_or(false);
        if placeholder_only {
            let _ = tree.clear_children(item);
        }
        if item.children() > 0 {
            return;
        }
        let Some(dir) = self.ui.paths.get(&key).cloned() else { return };
        for (name, sub) in library::child_dirs(&dir) {
            let child_key = format!("{key}/{name}");
            let has_sub = !library::child_dirs(&sub).is_empty();
            if let Some(mut child) = tree.add_to_parent(item, &name) {
                if has_sub {
                    let _ = tree.add_to_parent(&child, PLACEHOLDER);
                }
                child.close();
            }
            self.ui.paths.insert(child_key, sub);
        }
        self.refresh_dirs();
    }

    fn refresh_dirs(&mut self) {
        let mut dirs: Vec<PathBuf> = self.ui.paths.values().cloned().collect();
        dirs.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
        dirs.dedup();
        self.dirs = dirs;
    }

    /// Show a folder that lives outside the configured roots as a temporary root.
    fn ensure_tree_root(&mut self, dir: &Path) {
        if self.ui.paths.values().any(|p| p == dir) {
            return;
        }
        if self.s.roots.iter().any(|r| dir.starts_with(r)) {
            return;
        }
        let key = display_name(dir);
        let key = unique_key(&self.ui.paths, key);
        self.add_tree_node(None, key, dir, 1);
        self.refresh_dirs();
    }

    pub fn select_tree_for(&mut self, dir: &Path) {
        let key = self
            .ui
            .paths
            .iter()
            .find(|(_, p)| p.as_path() == dir)
            .map(|(k, _)| k.clone());
        let Some(key) = key else { return };
        let mut tree = self.ui.tree.clone();
        let _ = tree.select(&key, false);
        let parts: Vec<&str> = key.split('/').collect();
        for i in 1..parts.len() {
            let _ = tree.open(&parts[..=i].join("/"), false);
        }
        if let Some(item) = tree.find_item(&key) {
            self.expand_node(&item);
            let _ = tree.open(&key, false);
        }
    }

    pub fn select_tree_dir(&mut self, item: &TreeItem) {
        let Ok(key) = self.ui.tree.item_pathname(item) else { return };
        if let Some(dir) = self.ui.paths.get(&key).cloned() {
            self.load_dir(dir);
        }
    }

    pub fn add_root(&mut self, dir: PathBuf) {
        if !dir.is_dir() {
            return;
        }
        if !self.s.roots.contains(&dir) {
            self.s.roots.push(dir.clone());
        }
        let key = display_name(&dir);
        let key = unique_key(&self.ui.paths, key);
        self.add_tree_node(None, key.clone(), &dir, 1);
        self.refresh_dirs();
        let _ = self.ui.tree.select(&key, false);
        self.load_dir(dir);
        self.save_soon();
    }

    // ---- commands ----------------------------------------------------------

    pub fn cmd(&mut self, c: Cmd) {
        match c {
            Cmd::Menu => menus::main_menu(self, 20, 40),
            Cmd::Search | Cmd::Find => dialogs::find_files(self),
            Cmd::FindDrive => dialogs::find_files(self),
            Cmd::Explore => menus::explore(self),
            Cmd::Intro => {
                self.intro = Some(Intro {
                    start: Instant::now(),
                    per_track: Duration::from_secs(10),
                });
                if !self.is_playing() {
                    self.play_current(Duration::ZERO);
                }
                self.say("Intro scan: 10 seconds per track. Escape to stop.");
            }
            Cmd::App1 => dialogs::launch(self, 0),
            Cmd::App2 => dialogs::launch(self, 1),
            Cmd::App3 => dialogs::launch(self, 2),
            Cmd::Http => self.act(Act::HttpToggle),
            Cmd::Play => {
                let row = self.files().borrow().focus;
                self.play_row(row);
            }
            Cmd::StopResume => self.toggle_stop(),
            Cmd::Pause => self.toggle_pause(),
            Cmd::Prev => self.prev_track(),
            Cmd::Next => self.next_track(),
            Cmd::PrevDir => {
                self.goto_next_dir(false);
            }
            Cmd::NextDir => {
                self.goto_next_dir(true);
            }
            Cmd::Repeat => self.cycle_repeat(),
            Cmd::Shuffle => self.toggle_shuffle(),
            Cmd::Playlist => self.act(Act::TogglePlaylist),
            Cmd::TrackNext => self.track_next(),
            Cmd::Skip => self.remove_selected(),
            Cmd::Reload => {
                self.refresh_dir();
                self.rebuild_tree();
                self.say("Rescanned.");
            }
            Cmd::Home => self.go_home(),
            Cmd::Scan => self.scan_all(false),
            Cmd::Bookmarks => menus::bookmarks_menu(self, 0, 0),
            Cmd::Goto => dialogs::goto_dialog(self),
            Cmd::QuitNext => {
                self.quit_after = !self.quit_after;
                self.refresh_toolbar();
            }
            Cmd::BigView => self.act(Act::ToggleBig),
            Cmd::Log => self.act(Act::ViewLog),
            Cmd::Picture => self.act(Act::OpenPicture),
            Cmd::Mute => self.toggle_mute(),
            Cmd::Info => dialogs::track_info(self),
            Cmd::TrackHistory => menus::history_menu(self, true, 0, 0),
            Cmd::FolderHistory => menus::history_menu(self, false, 0, 0),
            Cmd::Settings => dialogs::settings_dialog(self),
            Cmd::Tree => self.act(Act::ToggleTree),
            Cmd::Compact => self.act(Act::ToggleCompact),
        }
    }

    pub fn act(&mut self, a: Act) {
        match a {
            Act::Settings => dialogs::settings_dialog(self),
            Act::About => dialogs::about_dialog(self),
            Act::Info => dialogs::track_info(self),
            Act::Find => dialogs::find_in_list(self),
            Act::FindFiles => dialogs::find_files(self),
            Act::Goto => dialogs::goto_dialog(self),
            Act::Refresh => {
                self.refresh_dir();
                self.rebuild_tree();
                self.say("Rescanned.");
            }
            Act::ToggleTree => {
                self.s.show_tree = !self.s.show_tree;
                self.apply_settings();
            }
            Act::TogglePlaylist => {
                self.s.playlist_open = !self.s.playlist_open;
                self.apply_settings();
            }
            Act::ToggleBig => {
                self.s.big_view = !self.s.big_view;
                self.apply_settings();
            }
            Act::ToggleCompact => {
                self.s.compact = !self.s.compact;
                if self.s.compact {
                    self.full_size = (self.ui.win.w(), self.ui.win.h());
                    self.ui.win.set_size(self.ui.win.w().max(520), 60);
                } else {
                    let (w, h) = self.full_size;
                    self.ui.win.set_size(w.max(640), h.max(360));
                }
                self.apply_settings();
            }
            Act::ToggleMute => self.toggle_mute(),
            Act::Pause => self.toggle_pause(),
            Act::Play => {
                let row = self.files().borrow().focus;
                self.play_row(row);
            }
            Act::PlayAt(i) => self.play_row(i),
            Act::SetRepeat(r) => {
                self.s.repeat = r;
                self.refresh_toolbar();
                self.save_soon();
            }
            Act::Stop => self.stop(),
            Act::Prev => self.prev_track(),
            Act::Next => self.next_track(),
            Act::PrevDir => {
                self.goto_next_dir(false);
            }
            Act::NextDir => {
                self.goto_next_dir(true);
            }
            Act::Repeat => self.cycle_repeat(),
            Act::Shuffle => self.toggle_shuffle(),
            Act::OneTrack => {
                self.s.one_track = !self.s.one_track;
                self.say(if self.s.one_track {
                    "Play one track."
                } else {
                    "Play whole list."
                });
            }
            Act::QuitTrack => {
                self.quit_after = !self.quit_after;
                self.refresh_toolbar();
            }
            Act::QuitEnd => {
                self.quit_at_end = !self.quit_at_end;
                self.refresh_flags();
            }
            Act::SortBy(k) => self.sort_by(k),
            Act::RemoveSelected => self.remove_selected(),
            Act::MoveSel(d) => self.move_selected(d),
            Act::TrackNext => self.track_next(),
            Act::Enqueue => self.enqueue_selected(),
            Act::SelectAll => {
                let len = self.files().borrow().tracks.len();
                {
                    let rc = self.files();
                    let mut m = rc.borrow_mut();
                    m.sel = (0..len).collect();
                }
                self.ui.files.redraw();
            }
            Act::ContentToPlaylist => self.content_to_playlist(),
            Act::Explore => menus::explore(self),
            Act::OpenStandard => {
                let tracks = self.selected_tracks();
                if tracks.is_empty() {
                    self.say("Select a file first.");
                } else {
                    for t in tracks {
                        dialogs::open_path(&t.path);
                    }
                }
            }
            Act::CopyPath => {
                let tracks = self.selected_tracks();
                let paths: Vec<String> = tracks
                    .iter()
                    .map(|t| t.path.to_string_lossy().into_owned())
                    .collect();
                if paths.is_empty() {
                    self.say("Select a file first.");
                } else {
                    app::copy(&paths.join("\n"));
                    self.say(format!(
                        "{} path{} copied.",
                        paths.len(),
                        if paths.len() == 1 { "" } else { "s" }
                    ));
                }
            }
            Act::Rename => {
                if let Some(t) = self.selected_track() {
                    dialogs::rename_dialog(self, &t.path);
                } else {
                    self.say("Select one file to rename.");
                }
            }
            Act::Delete => self.delete_selected(),
            Act::BookmarkTrack => self.bookmark_current(),
            Act::BookmarkFolder => self.bookmark_folder(),
            Act::ClearBookmarks => {
                self.s.bookmarks.clear();
                self.say("All bookmarks deleted.");
                self.save_soon();
            }
            Act::LoadBookmark(i) => self.load_bookmark(i),
            Act::LoadRecentTrack(i) => {
                let p = self.s.recent_tracks.get(i).cloned();
                if let Some(p) = p {
                    if let Some(dir) = p.parent() {
                        self.load_dir(dir.to_path_buf());
                        let idx = self.files().borrow().tracks.iter().position(|t| t.path == p);
                        if let Some(idx) = idx {
                            self.play_row(idx);
                        }
                    }
                }
            }
            Act::LoadRecentDir(i) => {
                let p = self.s.recent_dirs.get(i).cloned();
                if let Some(p) = p {
                    if p.is_dir() {
                        self.load_dir(p);
                    }
                }
            }
            Act::IntroScan => {
                self.cmd(Cmd::Intro);
            }
            Act::Scan => self.scan_all(false),
            Act::ViewLog => dialogs::log_dialog(self),
            Act::OpenPicture => {
                if let Some(t) = self.selected_track().or_else(|| self.track()) {
                    dialogs::open_path(&t.path);
                }
            }
            Act::HttpToggle => self.toggle_http(),
            Act::External(i) => dialogs::launch(self, i),
            Act::PlaylistPos(p) => {
                self.s.playlist_pos = p;
                self.apply_settings();
            }
            Act::ToggleAutoSearch => {
                self.s.auto_search_dirs = !self.s.auto_search_dirs;
                self.apply_settings();
            }
            Act::ToggleFocused => {
                self.s.continue_focused = !self.s.continue_focused;
                self.apply_settings();
            }
            Act::ToggleGapless => {
                self.s.gapless = !self.s.gapless;
                self.apply_settings();
            }
            Act::ToggleCover => {
                self.s.cover_art = !self.s.cover_art;
                self.apply_settings();
            }
            Act::NextPreset => self.next_preset(),
            Act::Visualizer => dialogs::visualizer_dialog(self),
            Act::VisualFull => self.toggle_visual_fullscreen(),
            Act::CoverMode(mode) => {
                self.s.cover_art = mode != CoverMode::Off;
                self.s.cover_mode = mode;
                self.apply_settings();
            }
            Act::ToggleEscQuit => {
                self.s.esc_quit = !self.s.esc_quit;
                self.apply_settings();
            }
            Act::ToggleLog => {
                self.s.log_enabled = !self.s.log_enabled;
                self.apply_settings();
            }
            Act::AddFolderRoot => {
                let mut d = fltk::dialog::FileDialog::new(fltk::dialog::FileDialogType::BrowseDir);
                d.set_title("Add folder");
                d.show();
                let p = d.filename();
                if !p.as_os_str().is_empty() {
                    self.add_root(p);
                }
            }
            Act::RemoveRoot => {
                if let Some(dir) = self.dir.clone() {
                    self.s.roots.retain(|r| *r != dir);
                    self.rebuild_tree();
                }
            }
            Act::LoadPath(s) => {
                let p = PathBuf::from(s);
                if p.is_dir() {
                    self.load_dir(p);
                } else if p.is_file() {
                    if let Some(dir) = p.parent() {
                        self.load_dir(dir.to_path_buf());
                        let idx = self.files().borrow().tracks.iter().position(|t| t.path == p);
                        if let Some(idx) = idx {
                            self.play_row(idx);
                        }
                    }
                }
            }
            Act::SavePlaylist => self.save_playlist(false),
            Act::AppendPlaylist => self.save_playlist(true),
            Act::Quit => {
                self.save();
                app::quit();
            }
        }
    }

    pub fn dispatch(&mut self) {
        loop {
            let msg = self.pending.borrow_mut().pop_front();
            let Some(msg) = msg else { break };
            match msg {
                Msg::Act(a) => self.act(a),
                Msg::Cmd(c) => self.cmd(c),
            }
        }
    }

    fn toggle_http(&mut self) {
        if self.http.is_some() {
            self.http = None;
            self.s.http_enabled = false;
            self.say("HTTP server stopped.");
        } else {
            match HttpServer::start(&self.s.http_addr) {
                Ok(s) => {
                    self.say(format!("HTTP server on http://{}", s.addr));
                    self.http = Some(s);
                    self.s.http_enabled = true;
                }
                Err(e) => {
                    self.s.http_enabled = false;
                    self.say(format!("HTTP server error: {e}"));
                }
            }
        }
        self.refresh_toolbar();
        self.save_soon();
    }

    pub fn go_home(&mut self) {
        let cur = self.cur;
        let h = self.ui.files.wid.h();
        {
            let rc = self.files();
            let mut m = rc.borrow_mut();
            m.sel.clear();
            m.sel.insert(cur);
            m.focus = cur;
            m.ensure_visible(cur, h);
        }
        self.focus = Focus::Files;
        self.ui.files.redraw();
        self.update_title();
    }

    pub fn load_bookmark(&mut self, i: usize) {
        let Some(b) = self.s.bookmarks.get(i).cloned() else { return };
        if b.path.is_dir() {
            self.load_dir(b.path);
        } else if b.path.is_file() {
            if let Some(dir) = b.path.parent() {
                self.load_dir(dir.to_path_buf());
                let idx = self.files().borrow().tracks.iter().position(|t| t.path == b.path);
                if let Some(idx) = idx {
                    self.cur = idx;
                    self.play_current(Duration::from_secs_f64(b.pos));
                }
            }
        }
    }

    fn save_playlist(&mut self, append: bool) {
        let rows = self.files().borrow().selected_rows();
        let tracks: Vec<Track> = {
            let m = self.files();
            let m = m.borrow();
            let rows = if rows.is_empty() {
                (0..m.tracks.len()).collect()
            } else {
                rows
            };
            rows.iter().filter_map(|r| m.tracks.get(*r).cloned()).collect()
        };
        if tracks.is_empty() {
            return;
        }
        let mut d = fltk::dialog::FileDialog::new(fltk::dialog::FileDialogType::BrowseSaveFile);
        d.set_title("Save playlist");
        d.set_filter("*.m3u");
        d.show();
        let path = d.filename();
        if path.as_os_str().is_empty() {
            return;
        }
        let mut text = String::from("#EXTM3U\n");
        if !append {
            text = String::from("#EXTM3U\n");
        }
        for t in &tracks {
            text.push_str(&format!("#EXTINF:-1,{}\n{}\n", t.artist_title(), t.path.display()));
        }
        let res = if append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut f| f.write_all(text.as_bytes()))
        } else {
            std::fs::write(&path, text)
        };
        match res {
            Ok(()) => self.say("Playlist saved."),
            Err(e) => self.say(format!("Playlist not saved: {e}")),
        }
    }

    pub fn bookmark_current(&mut self) {
        let tracks = self.selected_tracks();
        if tracks.is_empty() {
            self.say("Select a track first.");
            return;
        }
        let room = 30usize.saturating_sub(self.s.bookmarks.len());
        if room == 0 {
            self.say("Maximum number of bookmarks is 30.");
            return;
        }
        let playing = self.track().map(|t| t.path.clone());
        let pos = self.pos().as_secs_f64();
        let mut added = 0usize;
        for t in tracks.into_iter().take(room) {
            self.s.bookmarks.push(Bookmark {
                path: t.path.clone(),
                title: t.artist_title(),
                pos: if playing.as_ref() == Some(&t.path) {
                    pos
                } else {
                    0.0
                },
            });
            added += 1;
        }
        self.save_soon();
        self.say(format!(
            "{added} track{} added to bookmarks.",
            if added == 1 { "" } else { "s" }
        ));
    }

    pub fn bookmark_folder(&mut self) {
        let Some(dir) = self.dir.clone() else { return };
        if self.s.bookmarks.len() >= 30 {
            self.say("Maximum number of bookmarks is 30.");
            return;
        }
        self.s.bookmarks.push(Bookmark {
            path: dir.clone(),
            title: display_name(&dir),
            pos: 0.0,
        });
        self.save_soon();
        self.say("Folder added to bookmarks.");
    }

    pub fn on_tree(&mut self, reason: TreeReason, item: Option<TreeItem>) {
        match reason {
            TreeReason::Selected | TreeReason::Reselected => {
                if let Some(item) = item {
                    self.select_tree_dir(&item);
                }
            }
            TreeReason::Opened => {
                if let Some(item) = item {
                    self.expand_node(&item);
                }
            }
            _ => {}
        }
    }
}

/// Zero-width label used to mark a folder that has not been read yet.
const PLACEHOLDER: &str = "\u{200B}";

fn tracks_sig(tracks: &[Track]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for t in tracks {
        t.path.hash(&mut h);
        t.size.hash(&mut h);
        t.modified
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .hash(&mut h);
    }
    h.finish()
}

fn display_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| p.display().to_string())
}

fn unique_key(paths: &HashMap<String, PathBuf>, base: String) -> String {
    let mut key = base.clone();
    let mut n = 2;
    while paths.contains_key(&key) {
        key = format!("{base} ({n})");
        n += 1;
    }
    key
}

/// Keyboard shortcuts from the manual.
pub fn handle_key(a: &mut App) -> bool {
    let ctrl = app::event_state().contains(Shortcut::Command);
    let shift = app::event_state().contains(Shortcut::Shift);
    let alt = app::event_state().contains(Shortcut::Alt);
    let key = app::event_key();
    let ch = |c: char| Key::from_char(c);

    match (ctrl, shift, alt, key) {
        (false, false, false, Key::Enter) => {
            if a.focus == Focus::Files {
                let row = a.files().borrow().focus;
                a.play_row(row);
            }
            true
        }
        (false, false, false, Key::Escape) => {
            if a.intro.take().is_some() {
                a.say("Intro scan stopped.");
            } else if a.s.esc_quit {
                a.save();
                app::quit();
            }
            true
        }
        (false, false, false, k) if k == ch(' ') => {
            a.toggle_stop();
            true
        }
        (false, false, false, Key::Delete) => {
            a.remove_selected();
            true
        }
        (true, false, false, Key::Delete) => {
            a.delete_selected();
            true
        }
        (true, false, false, Key::BackSpace) => {
            a.delete_selected();
            true
        }
        (false, false, false, Key::Insert) => {
            a.track_next();
            true
        }
        (false, false, false, Key::F1) => {
            menus::main_menu(a, 20, 40);
            true
        }
        (false, false, false, Key::F2) => {
            if let Some(t) = a.selected_track() {
                dialogs::rename_dialog(a, &t.path);
            } else {
                a.say("Select one file to rename.");
            }
            true
        }
        (false, false, false, Key::F3) | (false, true, false, Key::F3) => {
            dialogs::find_files(a);
            true
        }
        (false, false, false, Key::F4) => {
            a.act(Act::ToggleTree);
            true
        }
        (false, false, false, Key::F5) => {
            a.act(Act::Refresh);
            true
        }
        (false, false, false, Key::F6) => {
            dialogs::settings_dialog(a);
            true
        }
        (false, false, false, Key::F7) => {
            a.act(Act::ToggleCompact);
            true
        }
        (false, false, false, Key::Tab) => {
            let open = a.s.playlist_open;
            let next = match a.focus {
                Focus::Files => {
                    if open {
                        Focus::Playlist
                    } else {
                        Focus::Files
                    }
                }
                Focus::Playlist => Focus::Files,
            };
            a.focus = next;
            let w = match next {
                Focus::Files => a.ui.files.wid.clone(),
                Focus::Playlist => a.ui.playlist.wid.clone(),
            };
            let mut w = w;
            let _ = w.take_focus();
            true
        }
        (false, false, false, Key::Pause) => {
            a.toggle_pause();
            true
        }
        (false, true, false, Key::Pause) => {
            a.toggle_mute();
            true
        }
        (false, true, false, Key::F4) => {
            a.scan_all(false);
            true
        }
        (false, true, false, Key::F5) => {
            a.act(Act::TogglePlaylist);
            true
        }
        (true, false, false, Key::Up) => {
            a.volume_step(true);
            true
        }
        (true, false, false, Key::Down) => {
            a.volume_step(false);
            true
        }
        (true, false, false, Key::Left) => {
            a.prev_track();
            true
        }
        (true, false, false, Key::Right) => {
            a.next_track();
            true
        }
        (false, true, false, Key::Left) => {
            a.goto_next_dir(false);
            true
        }
        (false, true, false, Key::Right) => {
            a.goto_next_dir(true);
            true
        }
        (false, false, false, Key::Left) => {
            a.rewind(false);
            true
        }
        (false, false, false, Key::Right) => {
            a.rewind(true);
            true
        }
        (true, false, false, Key::Enter) => {
            a.enqueue_selected();
            true
        }
        (true, false, false, Key::Home) => {
            a.go_home();
            true
        }
        (true, false, false, k) if k == ch('h') => {
            a.go_home();
            true
        }
        (true, false, false, k) if k == ch('r') => {
            a.cycle_repeat();
            true
        }
        (true, true, false, k) if k == ch('r') => {
            a.s.repeat = Repeat::One;
            a.refresh_toolbar();
            true
        }
        (true, false, false, k) if k == ch('b') => {
            a.act(Act::ToggleBig);
            true
        }
        (true, false, false, k) if k == ch('f') => {
            a.act(Act::Find);
            true
        }
        (true, false, false, k) if k == ch('g') => {
            a.act(Act::ToggleGapless);
            true
        }
        (true, false, false, k) if k == ch('i') => {
            a.act(Act::Info);
            true
        }
        (true, false, false, k) if k == ch('n') => {
            a.cmd(Cmd::Intro);
            true
        }
        (true, false, false, k) if k == ch('m') => {
            a.toggle_mute();
            true
        }
        (true, false, false, k) if k == ch('d') => {
            a.act(Act::ToggleAutoSearch);
            true
        }
        (true, false, false, k) if k == ch('q') => {
            a.save();
            app::quit();
            true
        }
        (true, true, false, k) if k == ch('q') => {
            a.act(Act::QuitTrack);
            true
        }
        (true, false, false, k) if k == ch('t') => {
            a.s.title_mode = match a.s.title_mode {
                TitleMode::Name => TitleMode::NameElapsed,
                TitleMode::NameElapsed => TitleMode::NameRemaining,
                TitleMode::NameRemaining => TitleMode::NameElapsedTotal,
                TitleMode::NameElapsedTotal => TitleMode::Name,
            };
            a.update_title();
            true
        }
        (true, false, false, k) if k == ch('x') => {
            menus::explore(a);
            true
        }
        (true, false, false, k) if k == ch('a') => {
            a.act(Act::SelectAll);
            true
        }
        (true, false, false, k) if k == ch('p') => {
            a.toggle_shuffle();
            true
        }
        (true, false, false, k) if k == ch('o') => {
            let next = match a.s.sort {
                SortKey::Name => SortKey::Modified,
                SortKey::Modified => SortKey::Size,
                SortKey::Size => SortKey::Type,
                SortKey::Type => SortKey::Created,
                SortKey::Created => SortKey::Length,
                SortKey::Length => SortKey::Name,
            };
            a.sort_by(next);
            true
        }
        (true, false, false, k) if k == ch('s') => {
            a.content_to_playlist();
            true
        }
        (true, false, false, k) if k == ch('u') => {
            a.next_track();
            true
        }
        (true, false, false, k) if k == ch('v') => {
            a.bookmark_current();
            true
        }
        (true, true, false, k) if k == ch('c') => {
            if let Some(t) = a.track() {
                app::copy(&t.path.to_string_lossy());
                a.say("Path copied.");
            }
            true
        }
        (true, true, false, k) if k == ch('l') => {
            a.act(Act::ViewLog);
            true
        }
        (true, true, false, k) if k == ch('g') => {
            a.act(Act::Goto);
            true
        }
        (true, true, false, k) if k == ch('p') => {
            a.act(Act::ToggleCover);
            true
        }
        (true, true, false, k) if k == ch('w') => {
            a.act(Act::HttpToggle);
            true
        }
        (true, true, false, k) if k == ch('o') => {
            a.act(Act::OpenStandard);
            true
        }
        (true, true, false, k) if k == ch('v') => {
            if let Some(t) = a.track() {
                dialogs::open_path(&t.path);
            }
            true
        }
        (true, true, false, k) if k == ch('e') => {
            a.say("Multiband enhancer is not part of this build.");
            true
        }
        (true, false, false, k) if k == ch('e') => {
            a.say("Enhancer is not part of this build.");
            true
        }
        (true, false, false, k) if k == ch('w') => {
            a.say("Equalizer is not part of this build.");
            true
        }
        _ => false,
    }
}
