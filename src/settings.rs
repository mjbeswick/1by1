use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    APP_NAME,
    logic::{Repeat, SortKey},
};

/// A stored track, folder, cue sheet or playlist. `pos` is used for tracks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub path: PathBuf,
    pub title: String,
    pub pos: f64,
}

/// Where the playlist view sits relative to the track list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaylistPos {
    #[default]
    Top,
    Bottom,
    Right,
}

#[allow(dead_code)]
impl PlaylistPos {
    pub fn label(self) -> &'static str {
        match self {
            PlaylistPos::Top => "Top",
            PlaylistPos::Bottom => "Bottom",
            PlaylistPos::Right => "Right",
        }
    }
}

/// How much information the big title view shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BigLayout {
    /// Title over times.
    #[default]
    TitleTimes,
    /// Title, then times, then info.
    TitleTimesInfo,
    /// Title and two info lines.
    TitleInfo2,
    /// Two info lines over the times.
    InfoTitleTimes,
}

impl BigLayout {
    pub fn label(self) -> &'static str {
        match self {
            BigLayout::TitleTimes => "1⁄2 title, times",
            BigLayout::TitleTimesInfo => "2⁄3 title, times, info",
            BigLayout::TitleInfo2 => "Title, info, info, times",
            BigLayout::InfoTitleTimes => "Title, times, info, info",
        }
    }
}

/// What the bottom left panel shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverMode {
    /// Folder or track artwork.
    #[default]
    Artwork,
    /// Milkdrop preset renderer.
    #[serde(alias = "spectrum")]
    Milkdrop,
    /// No panel.
    Off,
}

impl CoverMode {
    pub fn label(self) -> &'static str {
        match self {
            CoverMode::Artwork => "Artwork",
            CoverMode::Milkdrop => "Milkdrop",
            CoverMode::Off => "Off",
        }
    }
}

/// What the title bar shows besides the app name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleMode {
    /// File name or tag title.
    #[default]
    Name,
    /// Name plus elapsed time.
    NameElapsed,
    /// Name plus remaining time.
    NameRemaining,
    /// Name, elapsed and total.
    NameElapsedTotal,
}

/// Bitrate/info column content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InfoMode {
    #[default]
    Bitrate,
    ArtistTitle,
    TrackNo,
    Length,
    BitrateFmt,
    BitrateTag,
    YearAlbum,
    Notes,
}

impl InfoMode {
    pub const ALL: [InfoMode; 8] = [
        InfoMode::Bitrate,
        InfoMode::ArtistTitle,
        InfoMode::TrackNo,
        InfoMode::Length,
        InfoMode::BitrateFmt,
        InfoMode::BitrateTag,
        InfoMode::YearAlbum,
        InfoMode::Notes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InfoMode::Bitrate => "Bitrate",
            InfoMode::ArtistTitle => "Artist - Title",
            InfoMode::TrackNo => "Track number",
            InfoMode::Length => "Track length",
            InfoMode::BitrateFmt => "Bitrate, Samplerate, Channel",
            InfoMode::BitrateTag => "Bitrate, Tag type",
            InfoMode::YearAlbum => "Year, Album",
            InfoMode::Notes => "Notes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Session and resume.
    pub roots: Vec<PathBuf>,
    pub base_dir: Option<PathBuf>,
    pub last_dir: Option<PathBuf>,
    pub last_file: Option<PathBuf>,
    pub last_pos: f64,
    pub last_playlist: Vec<PathBuf>,
    /// Remembered position per folder, capped at ten folders.
    pub folder_state: Vec<(PathBuf, PathBuf, f64)>,
    pub volume: f32,
    pub muted: bool,
    pub repeat: Repeat,
    pub shuffle: bool,
    pub one_track: bool,
    pub sort: SortKey,
    pub sort_desc: bool,

    // Playback and control.
    pub auto_search_dirs: bool,
    pub continue_focused: bool,
    pub remember_positions: bool,
    pub unselect_on_change: bool,
    pub start_in_base: bool,
    pub skip_lists_in_search: bool,
    pub next_at_end_of_ffwd: bool,
    pub rewind_secs: u32,
    pub forward_secs: u32,
    pub resume_back_ms: u32,
    pub wheel_volume: bool,
    pub wheel_tree: bool,
    pub wheel_lists: bool,
    pub fine_volume: bool,
    pub larger_sliders: bool,
    pub show_tooltips: bool,
    pub tab_toolbar: bool,
    pub esc_quit: bool,
    pub gapless: bool,
    pub auto_space_ms: u32,
    pub fade_stop_ms: u32,
    pub fade_skip_ms: u32,
    pub go_home_on_change: bool,
    pub remove_played: bool,
    pub switch_to_folder_at_end: bool,
    pub reload_at_end: bool,
    pub no_label_edit: bool,

    // Lists and info.
    pub restore_list: bool,
    pub file_info_menu: bool,
    pub folders_in_list: bool,
    pub natural_sort: bool,
    pub lazy_tree: bool,
    pub extra_info_col: bool,
    pub info_mode: InfoMode,
    pub no_length_in_size: bool,
    pub always_scan: bool,
    pub hide_scrollbars: bool,

    // Window and list style.
    pub list_grid: bool,
    pub full_row: bool,
    pub tree_lines: bool,
    pub tree_spacing: bool,
    pub tree_style_explorer: bool,
    pub dim_inactive_list: u32,
    pub bigger_tree: bool,

    // Titles.
    pub title_mode: TitleMode,
    pub tag_titles: bool,
    pub upcoming_in_title: bool,
    pub colon_minutes: bool,

    // Bottom left panel.
    pub cover_art: bool,
    pub cover_mode: CoverMode,
    pub cover_hide_empty: bool,
    pub cover_names: String,

    // Draggable panel sizes.
    pub tree_width: i32,
    pub cover_height: i32,
    pub playlist_height: i32,
    pub playlist_width: i32,

    // Big view.
    pub big_view: bool,
    pub big_layout: BigLayout,
    pub big_idle_secs: u32,
    pub big_padding: u32,
    pub big_cycle_secs: u32,
    pub big_info_pattern: String,

    // Playlist.
    pub playlist_pos: PlaylistPos,
    pub playlist_open: bool,

    // System.
    pub one_instance: bool,
    pub tray_icon: bool,
    pub high_priority: bool,
    pub minimize_on_close: bool,
    pub autosave_5min: bool,
    pub proxy: String,
    pub http_enabled: bool,
    pub http_addr: String,

    // Logging and history.
    pub log_enabled: bool,
    pub log_max_kb: u32,
    pub history_menus: bool,
    pub history_entries: u32,
    pub recent_tracks: Vec<PathBuf>,
    pub recent_dirs: Vec<PathBuf>,

    // Launcher.
    pub external: [String; 3],

    // Bookmarks.
    pub bookmarks: Vec<Bookmark>,

    // UI session state.
    pub show_tree: bool,
    pub compact: bool,
    pub win_size: (i32, i32),
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            base_dir: None,
            last_dir: None,
            last_file: None,
            last_pos: 0.0,
            last_playlist: Vec::new(),
            folder_state: Vec::new(),
            volume: 0.85,
            muted: false,
            repeat: Repeat::Off,
            shuffle: false,
            one_track: false,
            sort: SortKey::Name,
            sort_desc: false,

            auto_search_dirs: false,
            continue_focused: false,
            remember_positions: true,
            unselect_on_change: false,
            start_in_base: false,
            skip_lists_in_search: false,
            next_at_end_of_ffwd: true,
            rewind_secs: 5,
            forward_secs: 5,
            resume_back_ms: 500,
            wheel_volume: true,
            wheel_tree: true,
            wheel_lists: true,
            fine_volume: false,
            larger_sliders: false,
            show_tooltips: true,
            tab_toolbar: true,
            esc_quit: false,
            gapless: false,
            auto_space_ms: 0,
            fade_stop_ms: 0,
            fade_skip_ms: 0,
            go_home_on_change: false,
            remove_played: false,
            switch_to_folder_at_end: false,
            reload_at_end: false,
            no_label_edit: false,

            restore_list: true,
            file_info_menu: true,
            folders_in_list: false,
            natural_sort: true,
            lazy_tree: true,
            extra_info_col: true,
            info_mode: InfoMode::ArtistTitle,
            no_length_in_size: false,
            always_scan: false,
            hide_scrollbars: false,

            list_grid: false,
            full_row: true,
            tree_lines: true,
            tree_spacing: false,
            tree_style_explorer: false,
            dim_inactive_list: 30,
            bigger_tree: false,

            title_mode: TitleMode::Name,
            tag_titles: true,
            upcoming_in_title: false,
            colon_minutes: true,

            cover_art: true,
            cover_mode: CoverMode::Artwork,
            cover_hide_empty: true,
            cover_names: "folder.jpg;folder.bmp;cover.jpg".into(),

            tree_width: 240,
            cover_height: 160,
            playlist_height: 160,
            playlist_width: 300,

            big_view: false,
            big_layout: BigLayout::TitleTimes,
            big_idle_secs: 0,
            big_padding: 12,
            big_cycle_secs: 10,
            big_info_pattern: "TITLE|INFO|TIME".into(),

            playlist_pos: PlaylistPos::Bottom,
            playlist_open: false,

            one_instance: true,
            tray_icon: false,
            high_priority: false,
            minimize_on_close: false,
            autosave_5min: true,
            proxy: String::new(),
            http_enabled: false,
            http_addr: "127.0.0.1:8080".into(),

            log_enabled: false,
            log_max_kb: 100,
            history_menus: false,
            history_entries: 10,
            recent_tracks: Vec::new(),
            recent_dirs: Vec::new(),

            external: [String::new(), String::new(), String::new()],

            bookmarks: Vec::new(),

            show_tree: true,
            compact: false,
            win_size: (960, 620),
        }
    }
}

/// Settings file. Portable mode keeps it next to the binary.
pub fn path() -> PathBuf {
    if let Some(local) = portable_path() {
        return local;
    }
    let base = env::var_os("APPDATA")
        .or_else(|| env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_NAME).join("settings.toml")
}

/// `settings.toml` beside the executable, used when it already exists or `/localini` is passed.
pub fn portable_path() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let p = exe.parent()?.join("settings.toml");
    (p.exists() || env::args().any(|a| a == "/localini")).then_some(p)
}

pub fn log_path(cfg: &Path) -> PathBuf {
    cfg.with_file_name("1by1.log")
}

/// Missing or broken file gives defaults.
pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Settings {
            roots: vec![PathBuf::from("/music/a")],
            last_file: Some(PathBuf::from("/music/a/x.mp3")),
            last_pos: 12.5,
            repeat: Repeat::All,
            shuffle: true,
            big_layout: BigLayout::TitleTimesInfo,
            ..Settings::default()
        };
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        assert_eq!(load(Path::new("/no/such/file.toml")), Settings::default());
    }
}
