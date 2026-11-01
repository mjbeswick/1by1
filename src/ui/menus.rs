#![allow(dead_code)]

use fltk::{
    app,
    enums::{FrameType, Key, Shortcut},
    menu::{MenuButton, MenuFlag, SysMenuBar, WindowMenuStyle},
    prelude::*,
};

use crate::{
    app::{Act, App, Focus, Msg},
    logic::{clock, size_text, SortKey},
    settings::CoverMode,
};

/// Add one entry that queues an action when clicked. Menu callbacks run inside a nested
/// event loop, so they must never borrow the app.
fn item<M: MenuExt>(a: &App, m: &mut M, label: &str, act: Act) {
    let pending = a.pending.clone();
    m.add(label, Shortcut::None, MenuFlag::Normal, move |_| {
        pending.borrow_mut().push_back(Msg::Act(act.clone()));
    });
}

fn item_sc<M: MenuExt>(a: &App, m: &mut M, label: &str, sc: Shortcut, act: Act) {
    let pending = a.pending.clone();
    m.add(label, sc, MenuFlag::Normal, move |_| {
        pending.borrow_mut().push_back(Msg::Act(act.clone()));
    });
}

fn sep<M: MenuExt>(m: &mut M) {
    // FLTK draws the divider below the item that carries the flag.
    // Adding an empty item instead leaves a blank clickable row.
    let n = m.size();
    if n > 0 {
        if let Some(mut prev) = m.at(n - 1) {
            prev.set_flag(MenuFlag::MenuDivider);
        }
    }
}

/// Main menu: F1 or a right click on the button bar.
pub fn main_menu(a: &mut App, x: i32, y: i32) {
    let (x, y) = place(a, x, y);
    let mut m = MenuButton::new(x, y, 0, 0, None);
    m.set_frame(FrameType::NoBox);
    item(a, &mut m, "Find in list...", Act::Find);
    item(a, &mut m, "Find files...", Act::FindFiles);
    item(a, &mut m, "Open folder, file, address...", Act::Goto);
    sep(&mut m);
    item(a, &mut m, "&Settings...", Act::Settings);
    item(a, &mut m, "Visualization...", Act::Visualizer);
    item(a, &mut m, "Big title view", Act::ToggleBig);
    item(a, &mut m, "Playlist view", Act::TogglePlaylist);
    item(a, &mut m, "Compact view", Act::ToggleCompact);
    sep(&mut m);
    item(a, &mut m, "Repeat/Off", Act::SetRepeat(crate::logic::Repeat::Off));
    item(a, &mut m, "Repeat/All tracks", Act::SetRepeat(crate::logic::Repeat::All));
    item(
        a,
        &mut m,
        "Repeat/Current track",
        Act::SetRepeat(crate::logic::Repeat::One),
    );
    item(a, &mut m, "Shuffle", Act::Shuffle);
    item(a, &mut m, "Play one track", Act::OneTrack);
    item(a, &mut m, "Auto search dirs", Act::ToggleAutoSearch);
    item(a, &mut m, "Gapless output", Act::ToggleGapless);
    item(a, &mut m, "Continue in focused list", Act::ToggleFocused);
    sep(&mut m);
    item(a, &mut m, "Quit after this track", Act::QuitTrack);
    item(a, &mut m, "Quit at end of list", Act::QuitEnd);
    sep(&mut m);
    item(a, &mut m, "Sort/Name", Act::SortBy(SortKey::Name));
    item(a, &mut m, "Sort/Size", Act::SortBy(SortKey::Size));
    item(a, &mut m, "Sort/Type", Act::SortBy(SortKey::Type));
    item(a, &mut m, "Sort/Modified", Act::SortBy(SortKey::Modified));
    item(a, &mut m, "Sort/Created", Act::SortBy(SortKey::Created));
    item(a, &mut m, "Sort/Length", Act::SortBy(SortKey::Length));
    sep(&mut m);
    bookmarks_submenu(a, &mut m);
    history_submenu(a, &mut m, true);
    history_submenu(a, &mut m, false);
    sep(&mut m);
    item(a, &mut m, "Track log...", Act::ViewLog);
    item(a, &mut m, "HTTP server", Act::HttpToggle);
    item(a, &mut m, "About 1by1", Act::About);
    item(a, &mut m, "Quit", Act::Quit);
    let _ = m.popup();
}

fn bookmarks_submenu(a: &App, m: &mut MenuButton) {
    item(a, m, "Bookmarks/Add current track", Act::BookmarkTrack);
    item(a, m, "Bookmarks/Add current folder", Act::BookmarkFolder);
    let list: Vec<String> = a
        .s
        .bookmarks
        .iter()
        .map(|b| format!("Bookmarks/{}", truncate(&b.title, 48)))
        .collect();
    for (i, label) in list.iter().enumerate() {
        item(a, m, label, Act::LoadBookmark(i));
    }
    item(a, m, "Bookmarks/Delete all", Act::ClearBookmarks);
}

fn history_submenu(a: &App, m: &mut MenuButton, tracks: bool) {
    let (prefix, list): (&str, Vec<String>) = if tracks {
        (
            "Recent tracks",
            a.s.recent_tracks
                .iter()
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .collect(),
        )
    } else {
        (
            "Recent folders",
            a.s.recent_dirs
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        )
    };
    if list.is_empty() {
        item(a, m, &format!("{prefix}/(empty)"), Act::Find);
        return;
    }
    for (i, label) in list.iter().enumerate() {
        let act = if tracks {
            Act::LoadRecentTrack(i)
        } else {
            Act::LoadRecentDir(i)
        };
        item(a, m, &format!("{prefix}/{}", truncate(label, 60)), act);
    }
}

/// Track context menu.
pub fn track_menu(a: &mut App, which: Focus, row: usize, x: i32, y: i32) {
    if which == Focus::Files {
        {
            let h = a.ui.files.wid.h();
            let rc = a.files();
            let mut m = rc.borrow_mut();
            if !m.sel.contains(&row) {
                m.sel.clear();
                m.sel.insert(row);
            }
            m.focus = row;
            m.ensure_visible(row, h);
        }
        a.ui.files.redraw();
    } else {
        a.focus = Focus::Playlist;
    }
    let sel_n = if which == Focus::Files {
        a.files().borrow().selected_rows().len()
    } else {
        1
    };
    let info = {
        let m = if which == Focus::Files {
            a.files()
        } else {
            a.playlist()
        };
        let m = m.borrow();
        m.tracks.get(row).map(|t| {
            let mut s = String::new();
            if let Some(d) = t.duration() {
                s.push_str(&format!("{}, ", clock(d)));
            }
            s.push_str(&size_text(t.size));
            if let Some(i) = &t.info {
                s.push_str(&format!(", {}", i.summary()));
            }
            if let Some(tag) = t.info.as_ref().and_then(|i| i.tags.artist_title()) {
                s.push_str(&format!("  |  {tag}"));
            }
            s
        })
    };
    let (x, y) = place(a, x, y);
    let mut m = MenuButton::new(x, y, 0, 0, None);
    m.set_frame(FrameType::NoBox);
    if a.s.file_info_menu {
        if let Some(info) = info {
            item(a, &mut m, &truncate(&info, 90), Act::Info);
            sep(&mut m);
        }
    }
    item(a, &mut m, "Track info...", Act::Info);
    if which == Focus::Files {
        item(a, &mut m, "Play", Act::PlayAt(row));
    } else {
        item(a, &mut m, "Play", Act::Enqueue);
    }
    item(a, &mut m, "Enqueue to playlist", Act::Enqueue);
    sep(&mut m);
    item(a, &mut m, "Skip (remove from list)", Act::RemoveSelected);
    item(a, &mut m, "Track = next", Act::TrackNext);
    item(a, &mut m, "Move up", Act::MoveSel(-1));
    item(a, &mut m, "Move down", Act::MoveSel(1));
    sep(&mut m);
    item(a, &mut m, "Explore folder", Act::Explore);
    item(a, &mut m, "Open standard", Act::OpenStandard);
    let copy = if sel_n > 1 {
        format!("Copy {sel_n} paths")
    } else {
        "Copy path".to_string()
    };
    item(a, &mut m, &copy, Act::CopyPath);
    item(a, &mut m, "Rename...", Act::Rename);
    let del = if sel_n > 1 {
        format!("Move {sel_n} files to Trash...")
    } else {
        "Move to Trash...".to_string()
    };
    item(a, &mut m, &del, Act::Delete);
    sep(&mut m);
    let bm = if sel_n > 1 {
        format!("Add {sel_n} to bookmarks")
    } else {
        "Add to bookmarks".to_string()
    };
    item(a, &mut m, &bm, Act::BookmarkTrack);
    item(a, &mut m, "Content to playlist", Act::ContentToPlaylist);
    item(a, &mut m, "Playlist file/Create...", Act::SavePlaylist);
    item(a, &mut m, "Playlist file/Append...", Act::AppendPlaylist);
    sep(&mut m);
    item(a, &mut m, "External 1", Act::External(0));
    item(a, &mut m, "External 2", Act::External(1));
    item(a, &mut m, "External 3", Act::External(2));
    let _ = m.popup();
}

/// Tree context menu.
pub fn tree_menu(a: &mut App, x: i32, y: i32) {
    let (x, y) = place(a, x, y);
    let mut m = MenuButton::new(x, y, 0, 0, None);
    m.set_frame(FrameType::NoBox);
    item(a, &mut m, "Explore folder", Act::Explore);
    item(a, &mut m, "Reload", Act::Refresh);
    sep(&mut m);
    item(a, &mut m, "Content to playlist", Act::ContentToPlaylist);
    item(a, &mut m, "Add folder to bookmarks", Act::BookmarkFolder);
    sep(&mut m);
    item(a, &mut m, "Add folder to tree...", Act::AddFolderRoot);
    item(a, &mut m, "Remove this root", Act::RemoveRoot);
    item(a, &mut m, "Settings...", Act::Settings);
    let _ = m.popup();
}

/// Context menu for the bottom left panel.
pub fn cover_menu(a: &mut App, x: i32, y: i32) {
    let (x, y) = place(a, x, y);
    let mut m = MenuButton::new(x, y, 0, 0, None);
    m.set_frame(FrameType::NoBox);
    item(a, &mut m, "Visualization manager...", Act::Visualizer);
    sep(&mut m);
    if a.s.cover_mode == CoverMode::Milkdrop {
        item(a, &mut m, "Next preset", Act::NextPreset);
        sep(&mut m);
    }
    item(a, &mut m, "Artwork", Act::CoverMode(CoverMode::Artwork));
    item(a, &mut m, "Milkdrop", Act::CoverMode(CoverMode::Milkdrop));
    sep(&mut m);
    item(a, &mut m, "Hide panel", Act::CoverMode(CoverMode::Off));
    let _ = m.popup();
}

/// Bookmarks button and menu entry.
pub fn bookmarks_menu(a: &mut App, x: i32, y: i32) {
    let (x, y) = place(a, x, y);
    let mut m = MenuButton::new(x, y, 0, 0, None);
    m.set_frame(FrameType::NoBox);
    item(a, &mut m, "Add current track", Act::BookmarkTrack);
    item(a, &mut m, "Add current folder", Act::BookmarkFolder);
    sep(&mut m);
    let list: Vec<String> = a
        .s
        .bookmarks
        .iter()
        .map(|b| truncate(&b.title, 60))
        .collect();
    if list.is_empty() {
        item(a, &mut m, "(no bookmarks)", Act::Find);
    }
    for (i, label) in list.iter().enumerate() {
        item(a, &mut m, label, Act::LoadBookmark(i));
    }
    sep(&mut m);
    item(a, &mut m, "Delete all bookmarks", Act::ClearBookmarks);
    let _ = m.popup();
}

pub fn history_menu(a: &mut App, tracks: bool, x: i32, y: i32) {
    let (x, y) = place(a, x, y);
    let mut m = MenuButton::new(x, y, 0, 0, None);
    m.set_frame(FrameType::NoBox);
    history_submenu(a, &mut m, tracks);
    let _ = m.popup();
}

/// Convert root (screen) coordinates into window coordinates for the menu popup.
fn place(a: &App, x: i32, y: i32) -> (i32, i32) {
    if x == 0 && y == 0 {
        return (24, 44);
    }
    let (ox, oy) = (a.ui.win.x_root(), a.ui.win.y_root());
    ((x - ox).max(0), (y - oy).max(0))
}

/// Open the folder of the current track in the system file manager.
pub fn explore(a: &mut App) {
    let mut dirs: Vec<std::path::PathBuf> = a
        .selected_tracks()
        .iter()
        .filter_map(|t| t.path.parent().map(|p| p.to_path_buf()))
        .collect();
    if dirs.is_empty() {
        if let Some(d) = a.dir.clone() {
            dirs.push(d);
        }
    }
    dirs.sort();
    dirs.dedup();
    for d in dirs {
        crate::ui::dialogs::open_path(&d);
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Right click on the toolbar or the lists.
pub fn position_from_event() -> (i32, i32) {
    (app::event_x_root(), app::event_y_root())
}

/// Install the native macOS menu bar. NoWindowMenu removes the empty Window menu.
pub fn install_sys_menu(a: &mut App) {
    SysMenuBar::set_window_menu_style(WindowMenuStyle::NoWindowMenu);
    let win = a.ui.win.clone();
    win.begin();
    let mut bar = SysMenuBar::default();
    bar.set_size(win.w(), 0);

    item_sc(a, &mut bar, "File/Open folder...", Shortcut::Command | Shortcut::Shift | 'g', Act::Goto);
    item_sc(a, &mut bar, "File/Find files...", Shortcut::from_key(Key::F3), Act::FindFiles);
    item_sc(a, &mut bar, "File/Reload", Shortcut::from_key(Key::F5), Act::Refresh);
    sep(&mut bar);
    item_sc(a, &mut bar, "File/Settings...", Shortcut::Command | ',', Act::Settings);
    item_sc(a, &mut bar, "File/Quit", Shortcut::Command | 'q', Act::Quit);

    item_sc(a, &mut bar, "Play/Play", Shortcut::None, Act::Play);
    item_sc(a, &mut bar, "Play/Stop", Shortcut::None, Act::Stop);
    item_sc(a, &mut bar, "Play/Pause", Shortcut::from_key(Key::Pause), Act::Pause);
    sep(&mut bar);
    item_sc(a, &mut bar, "Play/Previous track", Shortcut::Command | Shortcut::from_key(Key::Left), Act::Prev);
    item_sc(a, &mut bar, "Play/Next track", Shortcut::Command | Shortcut::from_key(Key::Right), Act::Next);
    item_sc(a, &mut bar, "Play/Previous folder", Shortcut::Command | Shortcut::Shift | Shortcut::from_key(Key::Left), Act::PrevDir);
    item_sc(a, &mut bar, "Play/Next folder", Shortcut::Command | Shortcut::Shift | Shortcut::from_key(Key::Right), Act::NextDir);
    sep(&mut bar);
    item_sc(a, &mut bar, "Play/Repeat/Off", Shortcut::None, Act::SetRepeat(crate::logic::Repeat::Off));
    item_sc(a, &mut bar, "Play/Repeat/All tracks", Shortcut::None, Act::SetRepeat(crate::logic::Repeat::All));
    item_sc(a, &mut bar, "Play/Repeat/Current track", Shortcut::None, Act::SetRepeat(crate::logic::Repeat::One));
    item_sc(a, &mut bar, "Play/Shuffle", Shortcut::Command | 'p', Act::Shuffle);
    item_sc(a, &mut bar, "Play/Play one track", Shortcut::None, Act::OneTrack);
    item_sc(a, &mut bar, "Play/Gapless output", Shortcut::Command | 'g', Act::ToggleGapless);
    item_sc(a, &mut bar, "Play/Auto search dirs", Shortcut::Command | 'd', Act::ToggleAutoSearch);
    item_sc(a, &mut bar, "Play/Quit after track", Shortcut::Command | Shortcut::Shift | 'q', Act::QuitTrack);

    item_sc(a, &mut bar, "View/Folder tree", Shortcut::from_key(Key::F4), Act::ToggleTree);
    item_sc(a, &mut bar, "View/Playlist", Shortcut::Command | Shortcut::Shift | Shortcut::from_key(Key::F5), Act::TogglePlaylist);
    item_sc(a, &mut bar, "View/Big title view", Shortcut::Command | 'b', Act::ToggleBig);
    item_sc(a, &mut bar, "View/Compact view", Shortcut::from_key(Key::F7), Act::ToggleCompact);
    item(a, &mut bar, "View/Visualization...", Act::Visualizer);
    item(a, &mut bar, "View/Full screen visualization", Act::VisualFull);
    sep(&mut bar);
    item_sc(a, &mut bar, "View/Find in list...", Shortcut::Command | 'f', Act::Find);
    item_sc(a, &mut bar, "View/Track info...", Shortcut::Command | 'i', Act::Info);

    item_sc(a, &mut bar, "Bookmarks/Add current track", Shortcut::None, Act::BookmarkTrack);
    item_sc(a, &mut bar, "Bookmarks/Add current folder", Shortcut::None, Act::BookmarkFolder);
    let list: Vec<String> = a.s.bookmarks.iter().map(|b| b.title.clone()).collect();
    for (i, title) in list.iter().enumerate() {
        let label = format!("Bookmarks/{}", title);
        item_sc(a, &mut bar, &label, Shortcut::None, Act::LoadBookmark(i));
    }
    if !list.is_empty() {
        sep(&mut bar);
        item_sc(a, &mut bar, "Bookmarks/Delete all", Shortcut::None, Act::ClearBookmarks);
    }

    item_sc(a, &mut bar, "Help/Track log", Shortcut::Command | Shortcut::Shift | 'l', Act::ViewLog);
    item_sc(a, &mut bar, "Help/HTTP server", Shortcut::Command | Shortcut::Shift | 'w', Act::HttpToggle);
    item_sc(a, &mut bar, "Help/About 1by1", Shortcut::None, Act::About);
    bar.set_about_callback(|_| {
        if let Some(rc) = GLOBAL_APP.with(|g| g.borrow().as_ref().and_then(|w| w.upgrade())) {
            if let Ok(mut a) = rc.try_borrow_mut() {
                a.act(Act::About);
            }
        }
    });
    win.end();
    bar.show();
    a.ui.menu_bar = Some(bar);
}

thread_local! {
    static GLOBAL_APP: std::cell::RefCell<Option<std::rc::Weak<std::cell::RefCell<App>>>> =
        const { std::cell::RefCell::new(None) };
}

pub fn remember_app(w: std::rc::Weak<std::cell::RefCell<App>>) {
    GLOBAL_APP.with(|g| *g.borrow_mut() = Some(w));
}
