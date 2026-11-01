#![allow(dead_code)]

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use fltk::{
    button::{Button, CheckButton},
    browser::HoldBrowser,
    dialog,
    enums::{Align, CallbackTrigger, Font},
    frame::Frame,
    group::Scroll,
    input::Input,
    menu::Choice,
    prelude::*,
    tree::{Tree, TreeReason},
    window::Window,
};

use crate::{
    app::App,
    library::{self, Track},
    logic,
    settings::{BigLayout, CoverMode, InfoMode, PlaylistPos, Settings, TitleMode},
    theme::Palette,
};

pub type Wa = std::rc::Weak<RefCell<App>>;

/// Escape closes a dialog window.
fn attach_esc(win: &mut Window) {
    let mut w = win.clone();
    win.handle(move |w2, ev| {
        if ev == fltk::enums::Event::KeyDown && fltk::app::event_key() == fltk::enums::Key::Escape {
            w2.hide();
            return true;
        }
        let _ = &mut w;
        false
    });
}

macro_rules! bind_check {
    ($w:expr, $wa:expr, $field:ident) => {{
        let wa = $wa.clone();
        $w.set_callback(move |c| {
            if let Some(a) = wa.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.s.$field = c.value();
                    a.apply_settings();
                    a.save_soon();
                }
            }
        });
    }};
}

macro_rules! bind_num {
    ($w:expr, $wa:expr, $field:ident, $ty:ty) => {{
        let wa = $wa.clone();
        $w.set_trigger(CallbackTrigger::Changed);
        $w.set_callback(move |c| {
            if let Ok(v) = c.value().trim().parse::<$ty>() {
                if let Some(a) = wa.upgrade() {
                    if let Ok(mut a) = a.try_borrow_mut() {
                        a.s.$field = v;
                        a.apply_settings();
                        a.save_soon();
                    }
                }
            }
        });
    }};
}

macro_rules! bind_text {
    ($w:expr, $wa:expr, $field:ident) => {{
        let wa = $wa.clone();
        $w.set_trigger(CallbackTrigger::Changed);
        $w.set_callback(move |c| {
            if let Some(a) = wa.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.s.$field = c.value();
                    a.apply_settings();
                    a.save_soon();
                }
            }
        });
    }};
}

fn label(x: i32, y: i32, w: i32, text: &str, pal: Palette) -> Frame {
    let mut f = Frame::new(x, y, w, 18, text);
    f.set_label_color(pal.text);
    f.set_label_size(12);
    f.set_align(Align::Inside | Align::Left);
    f
}

fn head(x: i32, y: &mut i32, text: &str, pal: Palette) {
    let mut f = Frame::new(x, *y, 460, 20, text);
    f.set_label_color(pal.text);
    f.set_label_size(12);
    f.set_label_font(Font::HelveticaBold);
    f.set_align(Align::Inside | Align::Left);
    *y += 28;
}

fn check(
    x: i32,
    y: &mut i32,
    text: &str,
    value: bool,
    pal: Palette,
    bind: impl FnOnce(&mut CheckButton),
) {
    let mut c = CheckButton::new(x, *y, 470, 20, text);
    c.set_value(value);
    c.set_label_color(pal.text);
    c.set_label_size(12);
    bind(&mut c);
    *y += 23;
}

fn num(
    x: i32,
    y: &mut i32,
    text: &str,
    value: String,
    pal: Palette,
    bind: impl FnOnce(&mut Input),
) {
    label(x, *y + 3, 250, text, pal);
    let mut i = Input::new(x + 262, *y, 70, 20, "");
    i.set_value(&value);
    bind(&mut i);
    *y += 28;
}

fn text_row(
    x: i32,
    y: &mut i32,
    text: &str,
    value: String,
    pal: Palette,
    bind: impl FnOnce(&mut Input),
) {
    label(x, *y + 3, 200, text, pal);
    let mut i = Input::new(x + 206, *y, 250, 20, "");
    i.set_value(&value);
    bind(&mut i);
    *y += 28;
}

fn pick(
    x: i32,
    y: &mut i32,
    text: &str,
    items: &[&str],
    value: i32,
    pal: Palette,
    bind: impl FnOnce(&mut Choice),
) {
    label(x, *y + 3, 200, text, pal);
    let mut c = Choice::new(x + 206, *y, 250, 22, None);
    for i in items {
        c.add(i, fltk::enums::Shortcut::None, fltk::menu::MenuFlag::Normal, |_| {});
    }
    c.set_value(value);
    bind(&mut c);
    *y += 30;
}

const SECTIONS: [&str; 11] = [
    "Playback",
    "Lists and info",
    "Window and knobs",
    "Titles and big view",
    "Control",
    "System",
    "Logging",
    "Launcher",
    "HTTP server",
    "Startup",
    "About",
];

/// The configuration window, opened with F6.
pub fn settings_dialog(a: &mut App) {
    let pal = a.pal;
    let wa = a.me.clone();
    let wa_save = a.me.clone();
    let mut win = Window::default()
        .with_size(760, 520)
        .with_label(" 1by1 Directory Player - Configuration");
    win.set_color(pal.window);

    let mut tree = Tree::new(10, 10, 200, 460, None);
    tree.set_show_root(false);
    tree.set_color(pal.list_bg);
    tree.set_selection_color(pal.list_sel);
    tree.set_item_label_fgcolor(pal.list_text);
    tree.set_item_label_bgcolor(pal.list_bg);
    tree.set_connector_color(pal.grid);
    tree.set_margin_top(4);
    tree.set_margin_left(10);
    for s in SECTIONS {
        tree.add(s);
    }
    let _ = tree.select(SECTIONS[0], false);

    let mut page = Scroll::new(220, 10, 520, 460, None);
    page.set_color(pal.panel);
    page.end();

    let mut close = Button::new(600, 480, 150, 28, "&Close");
    close.set_color(pal.button);
    close.set_label_color(pal.text);
    {
        let mut w = win.clone();
        let wa_close = wa_save.clone();
        close.set_callback(move |_| {
            w.hide();
            if let Some(a) = wa_close.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.save();
                }
            }
        });
    }

    let page_rc = Rc::new(RefCell::new(page.clone()));
    let wa2 = wa.clone();
    let page_cb = page_rc.clone();
    tree.set_callback(move |t| {
        if !matches!(
            t.callback_reason(),
            TreeReason::Selected | TreeReason::Reselected
        ) {
            return;
        }
        let Some(item) = t.callback_item() else { return };
        let Ok(name) = t.item_pathname(&item) else { return };
        let idx = SECTIONS.iter().position(|s| *s == name).unwrap_or(0);
        build_page(&page_cb, idx, &wa2);
    });

    win.end();
    attach_esc(&mut win);
    win.make_modal(true);
    win.show();
    build_page(&page_rc, 0, &wa);
}

#[allow(unused_assignments)]
fn build_page(page: &Rc<RefCell<Scroll>>, idx: usize, wa: &Wa) {
    let Some(app) = wa.upgrade() else { return };
    let (pal, s) = match app.try_borrow() {
        Ok(a) => (a.pal, a.s.clone()),
        Err(_) => (Palette::os(false), Settings::default()),
    };
    drop(app);

    let mut page = page.borrow_mut();
    page.clear();
    page.begin();
    page.set_color(pal.panel);
    let x = 236;
    let mut y = 16;

    match idx {
        0 => {
            head(x, &mut y, "Playback", pal);
            check(x, &mut y, "Auto search dirs", s.auto_search_dirs, pal, |c| {
                bind_check!(c, wa, auto_search_dirs)
            });
            check(
                x,
                &mut y,
                "Continue in focused list",
                s.continue_focused,
                pal,
                |c| bind_check!(c, wa, continue_focused),
            );
            check(
                x,
                &mut y,
                "Remember positions in ten folders",
                s.remember_positions,
                pal,
                |c| bind_check!(c, wa, remember_positions),
            );
            check(
                x,
                &mut y,
                "Start in base folder (on no param)",
                s.start_in_base,
                pal,
                |c| bind_check!(c, wa, start_in_base),
            );
            check(
                x,
                &mut y,
                "Dir search skips playlists and cue sheets",
                s.skip_lists_in_search,
                pal,
                |c| bind_check!(c, wa, skip_lists_in_search),
            );
            check(
                x,
                &mut y,
                "Next track if forward reaches the end",
                s.next_at_end_of_ffwd,
                pal,
                |c| bind_check!(c, wa, next_at_end_of_ffwd),
            );
            check(
                x,
                &mut y,
                "Gapless output (queue the next track)",
                s.gapless,
                pal,
                |c| bind_check!(c, wa, gapless),
            );
            y += 6;
            head(x, &mut y, "Skip times", pal);
            num(
                x,
                &mut y,
                "Rewind (left key, seconds):",
                s.rewind_secs.to_string(),
                pal,
                |i| bind_num!(i, wa, rewind_secs, u32),
            );
            num(
                x,
                &mut y,
                "Fast forward (right key, seconds):",
                s.forward_secs.to_string(),
                pal,
                |i| bind_num!(i, wa, forward_secs, u32),
            );
            num(
                x,
                &mut y,
                "Back on resume (millisecs):",
                s.resume_back_ms.to_string(),
                pal,
                |i| bind_num!(i, wa, resume_back_ms, u32),
            );
            num(
                x,
                &mut y,
                "Auto space between tracks (ms, 0 = off):",
                s.auto_space_ms.to_string(),
                pal,
                |i| bind_num!(i, wa, auto_space_ms, u32),
            );
        }
        1 => {
            head(x, &mut y, "Lists and folders", pal);
            check(
                x,
                &mut y,
                "Restore file list on start",
                s.restore_list,
                pal,
                |c| bind_check!(c, wa, restore_list),
            );
            check(
                x,
                &mut y,
                "File info in context menu",
                s.file_info_menu,
                pal,
                |c| bind_check!(c, wa, file_info_menu),
            );
            check(
                x,
                &mut y,
                "Folders in file list",
                s.folders_in_list,
                pal,
                |c| bind_check!(c, wa, folders_in_list),
            );
            check(
                x,
                &mut y,
                "Natural numerical ordering",
                s.natural_sort,
                pal,
                |c| bind_check!(c, wa, natural_sort),
            );
            check(
                x,
                &mut y,
                "Lazy folder tree (faster start)",
                s.lazy_tree,
                pal,
                |c| bind_check!(c, wa, lazy_tree),
            );
            check(
                x,
                &mut y,
                "Extra info column",
                s.extra_info_col,
                pal,
                |c| bind_check!(c, wa, extra_info_col),
            );
            let items: Vec<&str> = InfoMode::ALL.iter().map(|m| m.label()).collect();
            let cur = InfoMode::ALL
                .iter()
                .position(|m| *m == s.info_mode)
                .unwrap_or(0) as i32;
            pick(x, &mut y, "Info column shows:", &items, cur, pal, |c| {
                let wa = wa.clone();
                c.set_callback(move |c| {
                    if let Some(a) = wa.upgrade() {
                        if let Ok(mut a) = a.try_borrow_mut() {
                            if let Some(m) = InfoMode::ALL.get(c.value().max(0) as usize) {
                                a.s.info_mode = *m;
                                a.apply_settings();
                                a.save_soon();
                            }
                        }
                    }
                });
            });
            check(
                x,
                &mut y,
                "Always scan for times and info",
                s.always_scan,
                pal,
                |c| bind_check!(c, wa, always_scan),
            );
            y += 6;
            head(x, &mut y, "Track list style", pal);
            check(x, &mut y, "Gridlines in lists", s.list_grid, pal, |c| {
                bind_check!(c, wa, list_grid)
            });
            check(x, &mut y, "Full row select", s.full_row, pal, |c| {
                bind_check!(c, wa, full_row)
            });
            check(x, &mut y, "Show lines in the tree", s.tree_lines, pal, |c| {
                bind_check!(c, wa, tree_lines)
            });
            check(
                x,
                &mut y,
                "Add tree item spacing",
                s.tree_spacing,
                pal,
                |c| bind_check!(c, wa, tree_spacing),
            );
            check(
                x,
                &mut y,
                "Hide list scrollbars if possible",
                s.hide_scrollbars,
                pal,
                |c| bind_check!(c, wa, hide_scrollbars),
            );
        }
        2 => {
            head(x, &mut y, "Appearance", pal);
            let mut note = Frame::new(
                x,
                y,
                460,
                40,
                "Colors follow the system appearance. Switch macOS between light and dark \
                 and the window, lists, toolbar and icons follow within two seconds.",
            );
            note.set_label_color(pal.dim);
            note.set_align(Align::Inside | Align::Left | Align::Wrap);
            y += 48;
            check(
                x,
                &mut y,
                "Larger sliders",
                s.larger_sliders,
                pal,
                |c| bind_check!(c, wa, larger_sliders),
            );
            y += 6;
            head(x, &mut y, "Bottom left panel", pal);
            let cur = match s.cover_mode {
                CoverMode::Artwork => 0,
                CoverMode::Milkdrop => 1,
                CoverMode::Off => 2,
            };
            let labels = [
                CoverMode::Artwork.label(),
                CoverMode::Milkdrop.label(),
                CoverMode::Off.label(),
            ];
            pick(
                x,
                &mut y,
                "Show:",
                &labels,
                cur,
                pal,
                |c| {
                    let wa = wa.clone();
                    c.set_callback(move |c| {
                        if let Some(a) = wa.upgrade() {
                            if let Ok(mut a) = a.try_borrow_mut() {
                                a.s.cover_mode = match c.value() {
                                    1 => CoverMode::Milkdrop,
                                    2 => CoverMode::Off,
                                    _ => CoverMode::Artwork,
                                };
                                a.apply_settings();
                            }
                        }
                    });
                },
            );
            check(
                x,
                &mut y,
                "Hide the panel when there is no picture",
                s.cover_hide_empty,
                pal,
                |c| bind_check!(c, wa, cover_hide_empty),
            );
            text_row(
                x,
                &mut y,
                "Picture names (sep = ;):",
                s.cover_names.clone(),
                pal,
                |i| bind_text!(i, wa, cover_names),
            );
            y += 6;
            head(x, &mut y, "Window", pal);
            check(x, &mut y, "Show folder tree", s.show_tree, pal, |c| {
                bind_check!(c, wa, show_tree)
            });
            let items = ["Top", "Bottom", "Right"];
            let cur = match s.playlist_pos {
                PlaylistPos::Top => 0,
                PlaylistPos::Bottom => 1,
                PlaylistPos::Right => 2,
            };
            pick(x, &mut y, "Playlist position:", &items, cur, pal, |c| {
                let wa = wa.clone();
                c.set_callback(move |c| {
                    if let Some(a) = wa.upgrade() {
                        if let Ok(mut a) = a.try_borrow_mut() {
                            a.s.playlist_pos = match c.value() {
                                0 => PlaylistPos::Top,
                                2 => PlaylistPos::Right,
                                _ => PlaylistPos::Bottom,
                            };
                            a.apply_settings();
                            a.save_soon();
                        }
                    }
                });
            });
        }
        3 => {
            head(x, &mut y, "Title bar", pal);
            let items = [
                "Name",
                "Name and elapsed",
                "Name and remaining",
                "Name, elapsed, total",
            ];
            let cur = match s.title_mode {
                TitleMode::Name => 0,
                TitleMode::NameElapsed => 1,
                TitleMode::NameRemaining => 2,
                TitleMode::NameElapsedTotal => 3,
            };
            pick(x, &mut y, "Title bar shows:", &items, cur, pal, |c| {
                let wa = wa.clone();
                c.set_callback(move |c| {
                    if let Some(a) = wa.upgrade() {
                        if let Ok(mut a) = a.try_borrow_mut() {
                            a.s.title_mode = match c.value() {
                                1 => TitleMode::NameElapsed,
                                2 => TitleMode::NameRemaining,
                                3 => TitleMode::NameElapsedTotal,
                                _ => TitleMode::Name,
                            };
                            a.update_title();
                            a.save_soon();
                        }
                    }
                });
            });
            check(
                x,
                &mut y,
                "Tag titles instead of filename",
                s.tag_titles,
                pal,
                |c| bind_check!(c, wa, tag_titles),
            );
            check(
                x,
                &mut y,
                "Upcoming file in title bar",
                s.upcoming_in_title,
                pal,
                |c| bind_check!(c, wa, upcoming_in_title),
            );
            check(
                x,
                &mut y,
                "Minute symbol : instead of '",
                s.colon_minutes,
                pal,
                |c| bind_check!(c, wa, colon_minutes),
            );
            y += 6;
            head(x, &mut y, "Big view", pal);
            check(x, &mut y, "Show big title view", s.big_view, pal, |c| {
                bind_check!(c, wa, big_view)
            });
            let items: Vec<&str> = [
                BigLayout::TitleTimes,
                BigLayout::TitleTimesInfo,
                BigLayout::TitleInfo2,
                BigLayout::InfoTitleTimes,
            ]
            .iter()
            .map(|l| l.label())
            .collect();
            let cur = match s.big_layout {
                BigLayout::TitleTimes => 0,
                BigLayout::TitleTimesInfo => 1,
                BigLayout::TitleInfo2 => 2,
                BigLayout::InfoTitleTimes => 3,
            };
            pick(x, &mut y, "Layout:", &items, cur, pal, |c| {
                let wa = wa.clone();
                c.set_callback(move |c| {
                    if let Some(a) = wa.upgrade() {
                        if let Ok(mut a) = a.try_borrow_mut() {
                            a.s.big_layout = match c.value() {
                                1 => BigLayout::TitleTimesInfo,
                                2 => BigLayout::TitleInfo2,
                                3 => BigLayout::InfoTitleTimes,
                                _ => BigLayout::TitleTimes,
                            };
                            a.update_big();
                            a.save_soon();
                        }
                    }
                });
            });
            num(
                x,
                &mut y,
                "Inner padding (px):",
                s.big_padding.to_string(),
                pal,
                |i| bind_num!(i, wa, big_padding, u32),
            );
            num(
                x,
                &mut y,
                "Info cycle time (seconds):",
                s.big_cycle_secs.to_string(),
                pal,
                |i| bind_num!(i, wa, big_cycle_secs, u32),
            );
        }
        4 => {
            head(x, &mut y, "Keyboard and mouse", pal);
            check(
                x,
                &mut y,
                "Use wheel for volume",
                s.wheel_volume,
                pal,
                |c| bind_check!(c, wa, wheel_volume),
            );
            check(
                x,
                &mut y,
                "Wheel scrolls over the tree",
                s.wheel_tree,
                pal,
                |c| bind_check!(c, wa, wheel_tree),
            );
            check(
                x,
                &mut y,
                "Wheel scrolls over lists",
                s.wheel_lists,
                pal,
                |c| bind_check!(c, wa, wheel_lists),
            );
            check(
                x,
                &mut y,
                "Finer volume steps",
                s.fine_volume,
                pal,
                |c| bind_check!(c, wa, fine_volume),
            );
            check(x, &mut y, "Show tooltips", s.show_tooltips, pal, |c| {
                bind_check!(c, wa, show_tooltips)
            });
            check(
                x,
                &mut y,
                "Tab key access for the toolbar",
                s.tab_toolbar,
                pal,
                |c| bind_check!(c, wa, tab_toolbar),
            );
            check(x, &mut y, "Quit by Escape", s.esc_quit, pal, |c| {
                bind_check!(c, wa, esc_quit)
            });
            y += 6;
            head(x, &mut y, "Track change", pal);
            check(
                x,
                &mut y,
                "Go home before track change",
                s.go_home_on_change,
                pal,
                |c| bind_check!(c, wa, go_home_on_change),
            );
            check(
                x,
                &mut y,
                "Unselect selections on track change",
                s.unselect_on_change,
                pal,
                |c| bind_check!(c, wa, unselect_on_change),
            );
            check(
                x,
                &mut y,
                "Remove played tracks from list",
                s.remove_played,
                pal,
                |c| bind_check!(c, wa, remove_played),
            );
            check(
                x,
                &mut y,
                "On end of playlist switch to folder list",
                s.switch_to_folder_at_end,
                pal,
                |c| bind_check!(c, wa, switch_to_folder_at_end),
            );
            check(
                x,
                &mut y,
                "Reload directory at end of tracks",
                s.reload_at_end,
                pal,
                |c| bind_check!(c, wa, reload_at_end),
            );
        }
        5 => {
            head(x, &mut y, "Environment", pal);
            check(
                x,
                &mut y,
                "Only one instance allowed",
                s.one_instance,
                pal,
                |c| bind_check!(c, wa, one_instance),
            );
            check(
                x,
                &mut y,
                "Minimize instead of close",
                s.minimize_on_close,
                pal,
                |c| bind_check!(c, wa, minimize_on_close),
            );
            check(
                x,
                &mut y,
                "Auto save settings every five minutes",
                s.autosave_5min,
                pal,
                |c| bind_check!(c, wa, autosave_5min),
            );
            y += 6;
            head(x, &mut y, "Configuration files", pal);
            text_row(
                x,
                &mut y,
                "Proxy (empty = none):",
                s.proxy.clone(),
                pal,
                |i| bind_text!(i, wa, proxy),
            );
            let mut open = Button::new(x, y, 220, 24, "Explore settings folder");
            open.set_color(pal.button);
            let cfg = app_cfg(wa);
            open.set_callback(move |_| {
                if let Some(dir) = cfg.as_ref().and_then(|c| c.parent()) {
                    open_path(dir);
                }
            });
            y += 30;
        }
        6 => {
            head(x, &mut y, "Track logging", pal);
            check(x, &mut y, "Enable logging", s.log_enabled, pal, |c| {
                bind_check!(c, wa, log_enabled)
            });
            num(
                x,
                &mut y,
                "Maximum log file size (KB, 0 = endless):",
                s.log_max_kb.to_string(),
                pal,
                |i| bind_num!(i, wa, log_max_kb, u32),
            );
            check(
                x,
                &mut y,
                "Enable history menus",
                s.history_menus,
                pal,
                |c| bind_check!(c, wa, history_menus),
            );
            num(
                x,
                &mut y,
                "Entries (max 30):",
                s.history_entries.to_string(),
                pal,
                |i| bind_num!(i, wa, history_entries, u32),
            );
            let mut view = Button::new(x, y, 160, 24, "View track log");
            view.set_color(pal.button);
            let wa2 = wa.clone();
            view.set_callback(move |_| {
                if let Some(a) = wa2.upgrade() {
                    if let Ok(mut a) = a.try_borrow_mut() {
                        log_dialog(&mut a);
                    }
                }
            });
            y += 30;
        }
        7 => {
            head(x, &mut y, "Program launcher", pal);
            for (i, text) in [
                "External program 1:",
                "External program 2:",
                "External program 3:",
            ]
            .iter()
            .enumerate()
            {
                let value = s.external[i].clone();
                label(x, y + 3, 200, text, pal);
                let mut inp = Input::new(x + 206, y, 250, 20, "");
                inp.set_value(&value);
                inp.set_trigger(CallbackTrigger::Changed);
                let wa2 = wa.clone();
                inp.set_callback(move |c| {
                    if let Some(a) = wa2.upgrade() {
                        if let Ok(mut a) = a.try_borrow_mut() {
                            a.s.external[i] = c.value();
                            a.save_soon();
                        }
                    }
                });
                y += 24;
            }
            let mut hint = Frame::new(
                x,
                y,
                460,
                60,
                "Use %F for the selected file or folder. Empty entries hide the button.",
            );
            hint.set_label_color(pal.dim);
            hint.set_align(Align::Inside | Align::Left | Align::Wrap);
        }
        8 => {
            head(x, &mut y, "HTTP server", pal);
            check(
                x,
                &mut y,
                "Enable the web control server",
                s.http_enabled,
                pal,
                |c| bind_check!(c, wa, http_enabled),
            );
            text_row(
                x,
                &mut y,
                "Server address (ip:port):",
                s.http_addr.clone(),
                pal,
                |i| bind_text!(i, wa, http_addr),
            );
            let mut hint = Frame::new(
                x,
                y,
                460,
                90,
                "The server answers on /res /prv /nxt /rew /fwd /vdn /vup /dprv /dnxt /home and shows a small control page. Audio streaming is not part of this build.",
            );
            hint.set_label_color(pal.dim);
            hint.set_align(Align::Inside | Align::Left | Align::Wrap);
        }
        9 => {
            head(x, &mut y, "Startup", pal);
            check(
                x,
                &mut y,
                "Start in base folder (on no parameter)",
                s.start_in_base,
                pal,
                |c| bind_check!(c, wa, start_in_base),
            );
            check(
                x,
                &mut y,
                "Only one instance allowed",
                s.one_instance,
                pal,
                |c| bind_check!(c, wa, one_instance),
            );
            let mut hint = Frame::new(
                x,
                y,
                460,
                120,
                "Command line: 1by1 [file|folder|playlist] [/r /rt /rd /close /hide /enqueue /newinst /localini]",
            );
            hint.set_label_color(pal.dim);
            hint.set_align(Align::Inside | Align::Left | Align::Wrap);
        }
        _ => {
            head(x, &mut y, "About this build", pal);
            let mut f = Frame::new(
                x,
                y,
                460,
                380,
                "1by1 clone, written from scratch in Rust with FLTK.\n\n\
                 Clean room: behaviour comes from the public manual and the resource layout of the \
                 shipped program. No code, icons or text were copied.\n\n\
                 Not implemented: equalizer, the two enhancers, mono downmix, the MP3 streaming \
                 encoder, systray icon and global hotkeys.\n\n\
                 Formats: MP3, MP2, WAV, FLAC, OGG/Vorbis, AAC/MP4, AIFF.",
            );
            f.set_label_color(pal.text);
                    f.set_align(Align::Inside | Align::Left | Align::Top | Align::Wrap);
        }
    }
    page.end();
    page.redraw();
}

fn app_cfg(wa: &Wa) -> Option<PathBuf> {
    wa.upgrade()
        .and_then(|a| a.try_borrow().ok().map(|a| a.cfg.clone()))
}

/// Track information and tag editing.
pub fn track_info(a: &mut App) {
    let Some(track) = a.selected_track().or_else(|| a.track()) else {
        a.say("No track selected.");
        return;
    };
    if track.info.is_none() && !track.is_cue() {
        let info = library::scan_info(&track.path);
        let rc = a.files();
        let mut m = rc.borrow_mut();
        let pos = m
            .tracks
            .iter()
            .position(|t| t.path == track.path && t.cue == track.cue);
        if let Some(pos) = pos {
            m.tracks[pos].info = info;
        }
    }
    let track = a.selected_track().or_else(|| a.track()).unwrap_or(track);
    let pal = a.pal;
    let mut win = Window::default()
        .with_size(540, 540)
        .with_label("Track information");
    win.set_color(pal.window);

    let mut text = String::new();
    text.push_str(&format!("File: {}\n", track.path.display()));
    text.push_str(&format!("Size: {}\n", track.size_text()));
    if let Some(d) = track.duration() {
        text.push_str(&format!("Length: {}\n", logic::clock(d)));
    }
    if let Some(i) = &track.info {
        if let Some(f) = &i.format {
            text.push_str(&format!("Format: {f}\n"));
        }
        text.push_str(&format!("Audio: {}\n", i.summary()));
        if let Some(bits) = i.bits {
            text.push_str(&format!("Bits: {bits}\n"));
        }
        text.push_str(&format!("Decoder: {}\n", i.decoder()));
    }
    if let Some(m) = track.modified {
        text.push_str(&format!("Modified: {}\n", logic::datetime_text(m)));
    }
    if let Some(c) = track.created {
        text.push_str(&format!("Created: {}\n", logic::datetime_text(c)));
    }
    let mut info = Frame::new(12, 10, 516, 190, "");
    info.set_label(&text);
    info.set_label_color(pal.text);
    info.set_align(Align::Inside | Align::Left | Align::Top);
    info.set_label_size(12);

    let tags = track
        .info
        .as_ref()
        .map(|i| i.tags.clone())
        .unwrap_or_default();
    let mut y = 212;
    let mut fields: Vec<Input> = Vec::new();
    for (name, value) in [
        ("Title", tags.title.clone().unwrap_or_default()),
        ("Artist", tags.artist.clone().unwrap_or_default()),
        ("Album", tags.album.clone().unwrap_or_default()),
        ("Year", tags.year.clone().unwrap_or_default()),
        ("Track", tags.track.clone().unwrap_or_default()),
        ("Comment", tags.comment.clone().unwrap_or_default()),
    ] {
        label(12, y + 3, 70, name, pal);
        let mut i = Input::new(90, y, 290, 22, "");
        i.set_value(&value);
        fields.push(i);
        y += 27;
    }

    let mut save = Button::new(90, y + 8, 140, 26, "Save tags");
    save.set_color(pal.button);
    let mut close = Button::new(240, y + 8, 140, 26, "Close");
    let path = track.path.clone();
    let values: Vec<Input> = fields.clone();
    save.set_callback(move |_| {
        if library::is_module(&path) {
            dialog::message_default(
                "Tracker modules do not store editable tags. Title, author and message come from the module itself.",
            );
            return;
        }
        let tags = library::Tags {
            title: opt(values[0].value()),
            artist: opt(values[1].value()),
            album: opt(values[2].value()),
            year: opt(values[3].value()),
            track: opt(values[4].value()),
            comment: opt(values[5].value()),
        };
        match library::write_tags(&path, &tags) {
            Ok(()) => dialog::message_default("Tags saved. Reload the folder to refresh the list."),
            Err(e) => dialog::message_default(&format!("Could not save tags: {e}")),
        }
    });
    let mut w = win.clone();
    close.set_callback(move |_| {
        w.hide();
    });
    win.end();
    attach_esc(&mut win);
    win.make_modal(true);
    win.show();
}

fn opt(v: String) -> Option<String> {
    (!v.trim().is_empty()).then(|| v.trim().to_string())
}

/// Search within the current list.
pub fn find_in_list(a: &mut App) {
    let pal = a.pal;
    let wa = a.me.clone();
    let mut win = Window::default()
        .with_size(380, 116)
        .with_label("Find in list");
    win.set_color(pal.window);
    label(10, 12, 70, "Name:", pal);
    let mut input = Input::new(90, 10, 270, 22, "");
    let mut focus_input = input.clone();
    let mut next = Button::new(90, 44, 130, 26, "Find next");
    next.set_color(pal.button);
    let mut close = Button::new(230, 44, 130, 26, "Close");

    let mut input_cb = input.clone();
    let wa3 = wa.clone();
    let find = std::rc::Rc::new(move |input: &mut Input| {
        let needle = input.value();
        if needle.is_empty() {
            return;
        }
        if let Some(a) = wa3.upgrade() {
            if let Ok(mut a) = a.try_borrow_mut() {
                let len = a.files().borrow().tracks.len();
                if len == 0 {
                    return;
                }
                let start = {
                    let m = a.files();
                    let m = m.borrow();
                    (m.focus + 1) % len
                };
                let mut hit = None;
                for step in 0..len {
                    let row = (start + step) % len;
                    let name = a.files().borrow().tracks[row].name.clone();
                    if logic::contains_ci(&name, &needle) {
                        hit = Some(row);
                        break;
                    }
                }
                if let Some(row) = hit {
                    let h = a.ui.files.wid.h();
                    {
                        let rc = a.files();
                        let mut m = rc.borrow_mut();
                        m.sel.clear();
                        m.sel.insert(row);
                        m.focus = row;
                        m.ensure_visible(row, h);
                    }
                    a.ui.files.redraw();
                    let name = a.files().borrow().tracks[row].name.clone();
                    a.say(format!("Found: {name}"));
                } else {
                    a.say("Not found.");
                }
            }
        }
    });
    let f = find.clone();
    next.set_callback(move |_| f(&mut input_cb));
    input.set_trigger(CallbackTrigger::EnterKey);
    input.set_callback(move |i| find(i));
    let mut w = win.clone();
    close.set_callback(move |_| {
        w.hide();
    });
    win.end();
    attach_esc(&mut win);
    win.show();
    focus_input.take_focus().ok();
}

/// Recursive file finder. Results replace the file list.
pub fn find_files(a: &mut App) {
    let pal = a.pal;
    let wa = a.me.clone();
    let mut win = Window::default().with_size(480, 160).with_label("Find files");
    win.set_color(pal.window);
    label(10, 14, 100, "Name contains:", pal);
    let name = Input::new(116, 12, 340, 22, "");
    label(10, 44, 100, "Start folder:", pal);
    let mut start = Input::new(116, 42, 340, 22, "");
    start.set_value(
        &a.dir
            .as_ref()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let mut hint = Frame::new(10, 72, 440, 20, "The search always includes subfolders.");
    hint.set_label_color(pal.dim);
    let mut go = Button::new(116, 104, 120, 26, "Search");
    go.set_color(pal.button);
    let mut cancel = Button::new(246, 104, 120, 26, "Cancel");

    let w2 = win.clone();
    let wa2 = wa.clone();
    go.set_callback(move |_| {
        let needle = name.value();
        let root = PathBuf::from(start.value());
        if !root.is_dir() {
            dialog::message_default("That folder does not exist.");
            return;
        }
        let mut found = Vec::new();
        walk(&root, &needle, 0, &mut found);
        if let Some(a) = wa2.upgrade() {
            if let Ok(mut a) = a.try_borrow_mut() {
                let n = found.len();
                a.set_tracks(found, false);
                a.list_from_dir = false;
                a.say(format!("{n} files found."));
            }
        }
        let mut w = w2.clone();
        w.hide();
    });
    let mut w = win.clone();
    cancel.set_callback(move |_| {
        w.hide();
    });
    win.end();
    attach_esc(&mut win);
    win.show();
}

fn walk(dir: &std::path::Path, needle: &str, depth: usize, out: &mut Vec<Track>) {
    if depth > 12 || out.len() > 20_000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            walk(&p, needle, depth + 1, out);
        } else if library::is_audio(&p) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if needle.is_empty() || logic::contains_ci(&name, needle) {
                out.push(Track::new(p));
            }
        }
    }
}

fn fill_presets(
    list: &mut HoldBrowser,
    names: &[String],
    shown: &RefCell<Vec<u32>>,
    needle: &str,
) -> usize {
    list.clear();
    let mut idx = shown.borrow_mut();
    idx.clear();
    for (i, name) in names.iter().enumerate() {
        if needle.is_empty() || logic::contains_ci(name, needle) {
            idx.push(i as u32);
            list.add(name);
        }
    }
    if !idx.is_empty() {
        list.select(1);
    }
    idx.len()
}

/// Choose the panel mode and a Milkdrop preset.
pub fn visualizer_dialog(a: &mut App) {
    let pal = a.pal;
    let wa = a.me.clone();
    let has_engine = a.ensure_milkdrop();
    let names = Rc::new(a.preset_names());
    let mut win = Window::default()
        .with_size(620, 520)
        .with_label("Visualization");
    win.set_color(pal.window);

    label(12, 12, 60, "Mode:", pal);
    let mut mode = Choice::new(72, 10, 170, 24, "");
    mode.add_choice("Artwork|Milkdrop|Off");
    let modes = [
        CoverMode::Artwork,
        CoverMode::Milkdrop,
        CoverMode::Off,
    ];
    let mode_index = modes
        .iter()
        .position(|m| *m == a.s.cover_mode)
        .unwrap_or(0);
    mode.set_value(mode_index as i32);
    let wa_mode = wa.clone();
    mode.set_callback(move |c| {
        let Some(a) = wa_mode.upgrade() else { return };
        let Ok(mut a) = a.try_borrow_mut() else { return };
        let pick = match c.value() {
            1 => CoverMode::Milkdrop,
            2 => CoverMode::Off,
            _ => CoverMode::Artwork,
        };
        a.s.cover_art = pick != CoverMode::Off;
        a.s.cover_mode = pick;
        if pick == CoverMode::Milkdrop {
            a.ensure_milkdrop();
        }
        a.apply_settings();
    });

    let mut search = Input::new(72, 44, 440, 24, "");
    search.set_tooltip("Filter presets by name");
    let mut filter = Button::new(520, 44, 88, 24, "Filter");
    filter.set_color(pal.button);

    let mut list = fltk::browser::HoldBrowser::new(12, 78, 596, 372, "");
    list.set_scrollbar_size(14);
    list.set_color(pal.list_bg);
    list.set_selection_color(pal.list_sel);
    list.set_label_color(pal.list_text);
    list.set_text_size(12);

    let shown: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    fill_presets(&mut list, &names, &shown, "");

    let mut status = Frame::new(12, 458, 596, 18, "");
    status.set_label_color(pal.dim);
    status.set_label_size(11);
    status.set_align(Align::Inside | Align::Left);
    if has_engine {
        status.set_label(&format!("{} presets.", names.len()));
    } else {
        status.set_label("Milkdrop presets not found.");
    }

    let search2 = search.clone();
    let mut filter_list = list.clone();
    let filter_names = names.clone();
    let filter_shown = shown.clone();
    let mut filter_status = status.clone();
    filter.set_callback(move |_| {
        let n = fill_presets(
            &mut filter_list,
            &filter_names,
            &filter_shown,
            &search2.value(),
        );
        filter_status.set_label(&if n == 0 {
            "No presets match.".to_string()
        } else {
            format!("{n} presets.")
        });
    });
    let search3 = search.clone();
    let mut enter_list = list.clone();
    let enter_names = names.clone();
    let enter_shown = shown.clone();
    let mut enter_status = status.clone();
    search.set_trigger(CallbackTrigger::EnterKey);
    search.set_callback(move |_| {
        let n = fill_presets(
            &mut enter_list,
            &enter_names,
            &enter_shown,
            &search3.value(),
        );
        enter_status.set_label(&if n == 0 {
            "No presets match.".to_string()
        } else {
            format!("{n} presets.")
        });
    });

    let mut show_btn = Button::new(12, 482, 100, 26, "Show");
    show_btn.set_color(pal.button);
    let list2 = list.clone();
    let shown2 = shown.clone();
    let names2 = names.clone();
    let wa2 = wa.clone();
    let mut status2 = status.clone();
    show_btn.set_callback(move |_| {
        let row = list2.value();
        let index = if row > 0 {
            shown2.borrow().get((row - 1) as usize).copied()
        } else {
            None
        };
        let Some(index) = index else {
            status2.set_label("Select a preset first.");
            return;
        };
        let name = names2.get(index as usize).cloned().unwrap_or_default();
        let Some(a) = wa2.upgrade() else { return };
        let Ok(mut a) = a.try_borrow_mut() else { return };
        a.s.cover_art = true;
        a.s.cover_mode = CoverMode::Milkdrop;
        a.apply_settings();
        a.show_preset(index);
        status2.set_label(&format!("Preset: {name}"));
    });

    let mut next = Button::new(122, 482, 100, 26, "Next");
    let wa3 = wa.clone();
    let names3 = names.clone();
    let mut status3 = status.clone();
    next.set_callback(move |_| {
        let Some(a) = wa3.upgrade() else { return };
        let Ok(mut a) = a.try_borrow_mut() else { return };
        if !a.ensure_milkdrop() {
            status3.set_label("Milkdrop presets not found.");
            return;
        }
        a.s.cover_art = true;
        a.s.cover_mode = CoverMode::Milkdrop;
        a.apply_settings();
        a.next_preset();
        let i = a.milkdrop.as_ref().map(|m| m.current_preset()).unwrap_or(0);
        if let Some(name) = names3.get(i as usize) {
            status3.set_label(&format!("Preset: {name}"));
        }
    });

    let mut close = Button::new(508, 482, 100, 26, "Close");
    let mut w = win.clone();
    close.set_callback(move |_| {
        w.hide();
    });

    win.end();
    attach_esc(&mut win);
    win.show();
    search.take_focus().ok();
}

/// Go to a folder, file, playlist or address.
pub fn goto_dialog(a: &mut App) {
    let pal = a.pal;
    let wa = a.me.clone();
    let mut win = Window::default()
        .with_size(500, 96)
        .with_label("Open folder, file, address");
    win.set_color(pal.window);
    let mut input = Input::new(10, 12, 480, 24, "");
    let mut focus_input = input.clone();
    if let Some(d) = &a.dir {
        input.set_value(&d.to_string_lossy());
    }
    let mut ok = Button::new(320, 50, 80, 26, "OK");
    ok.set_color(pal.button);
    let mut cancel = Button::new(410, 50, 80, 26, "Cancel");
    let wa2 = wa.clone();
    let w2 = win.clone();
    let open = move |_: ()| {
        let p = PathBuf::from(input.value());
        if let Some(a) = wa2.upgrade() {
            if let Ok(mut a) = a.try_borrow_mut() {
                if p.is_dir() {
                    a.load_dir(p);
                    let mut w = w2.clone();
                    w.hide();
                } else if p.is_file() {
                    if let Some(dir) = p.parent() {
                        a.load_dir(dir.to_path_buf());
                        let idx = a
                            .files()
                            .borrow()
                            .tracks
                            .iter()
                            .position(|t| t.path == p);
                        if let Some(i) = idx {
                            a.play_row(i);
                        }
                        let mut w = w2.clone();
                        w.hide();
                    }
                } else {
                    a.say("Not found.");
                }
            }
        }
    };
    ok.set_callback(move |_| open(()));
    let mut w = win.clone();
    cancel.set_callback(move |_| {
        w.hide();
    });
    win.end();
    attach_esc(&mut win);
    win.show();
    focus_input.take_focus().ok();
}

pub fn about_dialog(a: &mut App) {
    let pal = a.pal;
    let mut win = Window::default().with_size(540, 380).with_label("About 1by1");
    win.set_color(pal.window);
    let mut f = Frame::new(16, 12, 508, 290, "");
    f.set_label_color(pal.text);
    f.set_align(Align::Inside | Align::Left | Align::Top | Align::Wrap);
    f.set_label(
        "1by1 clone\n\n\
         A directory audio player: no library, no playlists needed, each folder is the queue.\n\n\
         Clean-room remake in Rust with FLTK. Behaviour comes from the public manual and the \
         resource layout of the shipped program. Code, icons and text are new.\n\n\
         Shortcuts\n\
         Space stop / resume, Ctrl+R repeat, Ctrl+B big view, F4 tree, F5 reload,\n\
         F6 settings, F7 compact, Shift+F5 playlist, Ctrl+F find, F3 find files.",
    );
    let mut close = Button::new(220, 318, 100, 28, "Close");
    close.set_color(pal.button);
    let mut w = win.clone();
    close.set_callback(move |_| {
        w.hide();
    });
    win.end();
    attach_esc(&mut win);
    win.make_modal(true);
    win.show();
}

pub fn rename_dialog(a: &mut App, path: &std::path::Path) {
    let Some(current) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return;
    };
    let Some(new) = dialog::input_default("New name:", &current) else {
        return;
    };
    let new = new.trim().to_string();
    if new.is_empty() || new == current {
        return;
    }
    let target = path.with_file_name(&new);
    match std::fs::rename(path, &target) {
        Ok(()) => {
            a.refresh_dir();
            a.say("Renamed.");
        }
        Err(e) => a.say(format!("Rename failed: {e}")),
    }
}

pub fn log_dialog(a: &mut App) {
    let pal = a.pal;
    let path = crate::settings::log_path(&a.cfg);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        "Unable to load log file. Possible reasons: not found, too large, or empty.".into()
    });
    let mut win = Window::default().with_size(700, 460).with_label("Track log");
    win.set_color(pal.window);
    let mut out = fltk::text::TextDisplay::new(10, 10, 680, 400, "");
    let mut buf = fltk::text::TextBuffer::default();
    buf.set_text(&text);
    out.set_buffer(buf);
    out.set_text_size(12);
    let mut close = Button::new(590, 420, 100, 28, "Close");
    let mut w = win.clone();
    close.set_callback(move |_| {
        w.hide();
    });
    win.end();
    attach_esc(&mut win);
    win.make_modal(true);
    win.show();
}

/// Open a path with the system default program.
pub fn open_path(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let cmd = "xdg-open";
    let _ = std::process::Command::new(cmd).arg(path).spawn();
}

/// Run an external launcher with every selected file.
pub fn launch(a: &mut App, idx: usize) {
    let prog = a.s.external.get(idx).cloned().unwrap_or_default();
    if prog.trim().is_empty() {
        a.say("No external program configured.");
        return;
    }
    let files: Vec<String> = a
        .selected_tracks()
        .iter()
        .map(|t| t.path.to_string_lossy().into_owned())
        .collect();
    if files.is_empty() {
        a.say("Select a file first.");
        return;
    }
    for file in files {
        let parts: Vec<String> = prog
            .split_whitespace()
            .map(|p| p.replace("%F", &file))
            .collect();
        let Some((first, rest)) = parts.split_first() else {
            return;
        };
        let _ = std::process::Command::new(first).args(rest).spawn();
    }
}
