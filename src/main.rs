mod app;
mod http;
mod iconpack;
mod icons;
mod library;
mod logic;
mod milkdrop;
mod player;
mod settings;
mod theme;
mod ui;

use std::{
    cell::RefCell,
    collections::VecDeque,
    path::PathBuf,
    rc::Rc,
    time::Instant,
};

use fltk::{
    app::{self as fapp, App as FltkApp},
    enums::{Event, FrameType, Key, Shortcut},
    prelude::*,
};

use crate::app::{App, Focus, Msg};
use player::Audio;
use theme::Palette;

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn md_window_opaque(handle: *mut std::ffi::c_void);
}

pub const APP_NAME: &str = "1by1";

struct Args {
    path: Option<PathBuf>,
    resume: bool,
    resume_track: bool,
    resume_dir: bool,
    close_after: bool,
    hide: bool,
    enqueue: bool,
}

fn parse_args() -> Args {
    let mut out = Args {
        path: None,
        resume: false,
        resume_track: false,
        resume_dir: false,
        close_after: false,
        hide: false,
        enqueue: false,
    };
    for arg in std::env::args().skip(1) {
        match arg.to_ascii_lowercase().as_str() {
            "/r" => out.resume = true,
            "/rt" => out.resume_track = true,
            "/rd" => out.resume_dir = true,
            "/close" => out.close_after = true,
            "/hide" => out.hide = true,
            "/enqueue" => out.enqueue = true,
            "/newinst" | "/localini" => {}
            _ => {
                if arg.len() > 2 && arg.starts_with('/') && arg[1..].chars().all(|c| c.is_ascii_alphabetic()) {
                    continue;
                }
                out.path = Some(PathBuf::from(arg));
            }
        }
    }
    out
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {info}");
    }));
    let args = parse_args();
    let cfg = settings::path();
    let settings0 = settings::load(&cfg);
    let os_dark = theme::os_dark();
    let pal = Palette::os(os_dark);

    let mut fltk_app = FltkApp::default();
    fltk_app.set_scheme(fapp::Scheme::Gtk);
    let (pr, pg, pb) = pal.window.to_rgb();
    fapp::background(pr, pg, pb);
    fapp::set_visible_focus(false);
    fltk::misc::Tooltip::enable(true);
    fltk::misc::Tooltip::set_delay(0.45);

    let ui = ui::build(&settings0, pal);
    let audio = Audio::open();
    let pending: Rc<RefCell<VecDeque<Msg>>> = Rc::new(RefCell::new(VecDeque::new()));

    let mut state = App {
        s: settings0.clone(),
        cfg,
        pal,
        ui,
        dir: None,
        audio: None,
        cur: 0,
        focus: Focus::Files,
        queued_next: false,
        quit_after: false,
        quit_at_end: args.close_after,
        intro: None,
        log_lines: Vec::new(),
        last_save: Instant::now(),
        pending: pending.clone(),
        dirs: Vec::new(),
        status_msg: String::new(),
        dirty: false,
        http: None,
        me: std::rc::Weak::<RefCell<App>>::new(),
        started: Instant::now(),
        full_size: (settings0.win_size.0.max(640), settings0.win_size.1.max(360)),
        panel_drag: None,
        milkdrop: None,
        milkdrop_failed: false,
        visual_full: None,
        hover_hint: None,
        os_dark,
        last_theme_check: Instant::now(),
        last_fs_check: Instant::now(),
        list_sig: 0,
        list_from_dir: true,
    };
    if let Ok(a) = audio {
        state.audio = Some(a);
    } else if let Err(e) = audio {
        state.say(format!("No audio output: {e}"));
    }

    // Restore the saved playlist.
    let saved_playlist = state.s.last_playlist.clone();
    if !saved_playlist.is_empty() {
        let tracks: Vec<library::Track> = saved_playlist
            .iter()
            .filter(|p| p.is_file())
            .map(|p| library::Track::new(p.clone()))
            .collect();
        let mut m = state.ui.playlist.model.borrow_mut();
        m.tracks = tracks;
        m.sel.clear();
    }
    let playlist_open = state.s.playlist_open;

    state.ui.relayout(&state.s);
    state.ui.vol.set(state.s.volume as f64);
    state.set_theme();
    state.apply_list_style_public();
    state.refresh_toolbar();
    state.ui.win.set_label(APP_NAME);
    state.s.playlist_open = playlist_open;
    state.ui.win.end();
    state.ui.win.set_frame(FrameType::FlatBox);

    let shared: Rc<RefCell<App>> = Rc::new(RefCell::new(state));
    shared.borrow_mut().me = Rc::downgrade(&shared);

    install_callbacks(&shared);
    {
        let mut a = shared.borrow_mut();
        crate::ui::menus::install_sys_menu(&mut a);
    }
    crate::ui::menus::remember_app(Rc::downgrade(&shared));

    let s0 = shared.borrow().s.clone();
    {
        let mut a = shared.borrow_mut();
        if a.s.roots.is_empty() {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"));
            a.s.roots.push(home.clone());
            a.s.base_dir = Some(home);
        }
        a.rebuild_tree();
    }

    let mut start_dir = args
        .path
        .clone()
        .filter(|p| p.is_dir())
        .or_else(|| args.path.as_ref().and_then(|p| p.parent().map(|p| p.to_path_buf())));
    if start_dir.is_none() && (s0.start_in_base) {
        start_dir = s0.base_dir.clone();
    }
    if start_dir.is_none() {
        start_dir = s0.last_dir.clone();
    }
    let start_dir = start_dir
        .or_else(|| s0.base_dir.clone())
        .or_else(|| shared.borrow().s.roots.first().cloned());

    {
        let mut a = shared.borrow_mut();
        if let Some(dir) = start_dir {
            a.load_dir(dir);
        }
        if let Some(file) = args.path.clone().filter(|p| p.is_file()) {
            let idx = a
                .files()
                .borrow()
                .tracks
                .iter()
                .position(|t| t.path == file);
            if let Some(i) = idx {
                a.cur = i;
                if args.enqueue {
                    a.enqueue_selected();
                } else {
                    let seek = if args.resume && !args.resume_track {
                        std::time::Duration::from_secs_f64(a.s.last_pos)
                    } else {
                        std::time::Duration::ZERO
                    };
                    a.play_current(seek);
                }
            }
        } else if args.resume || args.resume_track {
            let seek = if args.resume_track {
                std::time::Duration::ZERO
            } else {
                std::time::Duration::from_secs_f64(a.s.last_pos)
            };
            a.play_current(seek);
        }
        if a.s.http_enabled {
            a.act(crate::app::Act::HttpToggle);
        }
        a.refresh_toolbar();
        a.update_title();
    }

    {
        let mut a = shared.borrow_mut();
        if args.hide {
            a.ui.win.hide();
        } else {
            a.ui.win.show();
            #[cfg(target_os = "macos")]
            unsafe {
                md_window_opaque(a.ui.win.raw_handle() as *mut std::ffi::c_void);
            }
        }
    }

    let s1 = shared.clone();
    fapp::add_timeout3(0.03, move |h| {
        if let Ok(mut a) = s1.try_borrow_mut() {
            a.dispatch();
        }
        fapp::repeat_timeout3(0.03, h);
    });

    let s2 = shared.clone();
    fapp::add_timeout3(0.12, move |h| {
        if let Ok(mut a) = s2.try_borrow_mut() {
            a.tick();
        }
        fapp::repeat_timeout3(0.12, h);
    });

    let sv = shared.clone();
    fapp::add_timeout3(0.066, move |h| {
        if let Ok(mut a) = sv.try_borrow_mut() {
            match a.s.cover_mode {
                crate::settings::CoverMode::Milkdrop => a.tick_visualizer(),
                _ => {}
            }
        }
        fapp::repeat_timeout3(0.066, h);
    });

    if std::env::var_os("ONEBY1_SMOKE").is_some() {
        let s = shared.clone();
        fapp::add_timeout3(0.6, move |h| {
            if let Ok(mut a) = s.try_borrow_mut() {
                let row = a.files().borrow().focus;
                a.play_row(row);
            }
            fapp::repeat_timeout3(0.2, h);
        });
        let s3 = shared.clone();
        fapp::add_timeout3(1.2, move |_| {
            if let Ok(mut a) = s3.try_borrow_mut() {
                a.scan_all(false);
                a.act(crate::app::Act::TogglePlaylist);
                a.act(crate::app::Act::ToggleBig);
                println!(
                    "layout: files={:?} playlist={:?} big={:?}",
                    (a.ui.files.wid.x(), a.ui.files.wid.y(), a.ui.files.wid.w(), a.ui.files.wid.h()),
                    (a.ui.playlist.wid.x(), a.ui.playlist.wid.y(), a.ui.playlist.wid.w(), a.ui.playlist.wid.h()),
                    (a.ui.big.wid.x(), a.ui.big.wid.y(), a.ui.big.wid.w(), a.ui.big.wid.h())
                );
            }
        });
        let s2 = shared.clone();
        fapp::add_timeout3(6.0, move |_| {
            if let Ok(mut a) = s2.try_borrow_mut() {
                println!(
                    "smoke: playing={} pos={:.2} status={} file={}",
                    a.is_playing(),
                    a.pos().as_secs_f32(),
                    a.status_msg,
                    a.track().map(|t| t.name).unwrap_or_default()
                );
                a.save();
            }
            fapp::quit();
        });
    }

    // Register a global key handler so shortcuts work for every focused widget.
    {
        let weak = Rc::downgrade(&shared);
        GLOBAL.with(|g| *g.borrow_mut() = Some(weak));
        fapp::add_handler(global_key);
    }

    {
        let s = shared.clone();
        fapp::add_timeout3(0.25, move |_| {
            if let Ok(mut a) = s.try_borrow_mut() {
                let _ = a.ui.files.wid.take_focus();
            }
        });
    }

    fltk_app.run().unwrap();
    if let Ok(mut a) = shared.try_borrow_mut() {
        a.save();
    }
}

fn install_callbacks(shared: &Rc<RefCell<App>>) {
    {
        let w = Rc::downgrade(shared);
        shared.borrow().ui.bar.on_cmd(move |cmd| {
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.cmd(cmd);
                }
            }
        });
    }
    {
        let w = Rc::downgrade(shared);
        shared.borrow().ui.bar.on_hover(move |hint| {
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.set_hover_hint(hint);
                }
            }
        });
    }
    {
        let w = Rc::downgrade(shared);
        shared.borrow().ui.files.on_event(move |ev| {
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.handle_list(Focus::Files, ev);
                }
            }
        });
    }
    {
        let w = Rc::downgrade(shared);
        shared.borrow().ui.playlist.on_event(move |ev| {
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.handle_list(Focus::Playlist, ev);
                }
            }
        });
    }
    {
        let w = Rc::downgrade(shared);
        shared.borrow().ui.seek.on_change(move |v| {
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.seek_to(v);
                }
            }
        });
    }
    {
        let w = Rc::downgrade(shared);
        shared.borrow().ui.vol.on_change(move |v| {
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.set_volume(v);
                }
            }
        });
    }

    install_split(shared, crate::ui::SplitKind::Tree);
    install_split(shared, crate::ui::SplitKind::Cover);
    install_split(shared, crate::ui::SplitKind::Playlist);

    {
        let w = Rc::downgrade(shared);
        let mut cover = shared.borrow().ui.cover.wid.clone();
        cover.handle(move |_, ev| {
            if ev == Event::Push && fapp::event_button() == 1 && fapp::event_clicks() {
                if let Some(a) = w.upgrade() {
                    if let Ok(mut a) = a.try_borrow_mut() {
                        a.toggle_visual_fullscreen();
                        return true;
                    }
                }
            }
            if ev == Event::Released && fapp::event_button() == 3 {
                if let Some(a) = w.upgrade() {
                    if let Ok(mut a) = a.try_borrow_mut() {
                        let (x, y) = (fapp::event_x_root(), fapp::event_y_root());
                        crate::ui::menus::cover_menu(&mut a, x, y);
                        return true;
                    }
                }
            }
            false
        });
    }

    // Folder tree selection and lazy expansion.
    {
        let w = Rc::downgrade(shared);
        let mut tree = shared.borrow().ui.tree.clone();
        tree.set_callback(move |t| {
            let reason = t.callback_reason();
            let item = t.callback_item();
            if let Some(a) = w.upgrade() {
                if let Ok(mut a) = a.try_borrow_mut() {
                    a.on_tree(reason, item);
                }
            }
        });
    }

    // Window events: shortcuts, layout, wheel over the toolbar, close.
    {
        let w = Rc::downgrade(shared);
        let mut win = shared.borrow().ui.win.clone();
        win.handle(move |win, ev| {
            let Some(rc) = w.upgrade() else { return false };
            match ev {
                Event::KeyDown => {
                    if let Ok(mut a) = rc.try_borrow_mut() {
                        return crate::app::handle_key(&mut a);
                    }
                    false
                }
                Event::Resize => {
                    if let Ok(mut a) = rc.try_borrow_mut() {
                        let st = a.s.clone();
                        a.ui.relayout(&st);
                        a.ui.win.redraw();
                    }
                    false
                }
                Event::MouseWheel => {
                    let (my, dy) = (fapp::event_y(), fapp::event_dy_value());
                    if my < 40 {
                        if let Ok(mut a) = rc.try_borrow_mut() {
                            if a.s.wheel_volume {
                                if dy > 0 {
                                    a.volume_step(true);
                                } else if dy < 0 {
                                    a.volume_step(false);
                                }
                                return true;
                            }
                        }
                    }
                    false
                }
                Event::Push if fapp::event_button() == 3 && fapp::event_y() < 40 => {
                    if let Ok(mut a) = rc.try_borrow_mut() {
                        let (x, y) = (fapp::event_x_root(), fapp::event_y_root());
                        crate::ui::menus::main_menu(&mut a, x, y);
                        return true;
                    }
                    false
                }
                Event::Paste => {
                    let text = fapp::event_text();
                    if let Ok(mut a) = rc.try_borrow_mut() {
                        drop_files(&mut a, &text);
                        return true;
                    }
                    false
                }
                Event::DndEnter | Event::DndDrag | Event::DndRelease => true,
                Event::Shortcut => {
                    if fapp::event_state().contains(Shortcut::Command) {
                        false
                    } else {
                        false
                    }
                }
                Event::Close => {
                    if let Ok(mut a) = rc.try_borrow_mut() {
                        if a.s.minimize_on_close {
                            win.hide();
                            return true;
                        }
                        a.save();
                    }
                    fapp::quit();
                    true
                }
                _ => false,
            }
        });
    }
}

fn install_split(shared: &Rc<RefCell<App>>, kind: crate::ui::SplitKind) {
    let mut wid = {
        let a = shared.borrow();
        match kind {
            crate::ui::SplitKind::Tree => a.ui.tree_split.wid.clone(),
            crate::ui::SplitKind::Cover => a.ui.cover_split.wid.clone(),
            crate::ui::SplitKind::Playlist => a.ui.playlist_split.wid.clone(),
        }
    };
    let weak = Rc::downgrade(shared);
    wid.handle(move |_, ev| match ev {
        Event::Enter => {
            if let Some(rc) = weak.upgrade() {
                if let Ok(a) = rc.try_borrow() {
                    let cursor = match kind {
                        crate::ui::SplitKind::Tree => fltk::enums::Cursor::WE,
                        crate::ui::SplitKind::Cover => fltk::enums::Cursor::NS,
                        crate::ui::SplitKind::Playlist => {
                            if a.s.playlist_pos == crate::settings::PlaylistPos::Right {
                                fltk::enums::Cursor::WE
                            } else {
                                fltk::enums::Cursor::NS
                            }
                        }
                    };
                    fltk::draw::set_cursor(cursor);
                    return true;
                }
            }
            false
        }
        Event::Leave => {
            fltk::draw::set_cursor(fltk::enums::Cursor::Default);
            true
        }
        Event::Push if fapp::event_button() == 1 => {
            if let Some(rc) = weak.upgrade() {
                if let Ok(mut a) = rc.try_borrow_mut() {
                    a.panel_press(kind, fapp::event_x_root(), fapp::event_y_root());
                    return true;
                }
            }
            false
        }
        Event::Drag => {
            if let Some(rc) = weak.upgrade() {
                if let Ok(mut a) = rc.try_borrow_mut() {
                    a.panel_drag_to(fapp::event_x_root(), fapp::event_y_root());
                    return true;
                }
            }
            false
        }
        Event::Released => {
            if let Some(rc) = weak.upgrade() {
                if let Ok(mut a) = rc.try_borrow_mut() {
                    a.panel_release();
                    return true;
                }
            }
            false
        }
        _ => false,
    });
}

thread_local! {
    static GLOBAL: RefCell<Option<std::rc::Weak<RefCell<App>>>> = const { RefCell::new(None) };
}

/// Keys handled before the focused widget: function keys and modifier combinations.
fn global_key(ev: Event) -> bool {
    if ev != Event::KeyDown {
        return false;
    }
    #[allow(unused)]
    let key = fapp::event_key();
    let state = fapp::event_state();
    let modified = state.contains(Shortcut::Command) || state.contains(Shortcut::Alt);
    let function = matches!(
        key,
        Key::F1 | Key::F2 | Key::F3 | Key::F4 | Key::F5 | Key::F6 | Key::F7 | Key::F8 | Key::F9
            | Key::F10 | Key::F11 | Key::F12
    );
    let plain = !modified
        && matches!(
            key,
            Key::Enter | Key::Delete | Key::Insert | Key::Escape | Key::Pause
        );
    if !(modified || function || plain) {
        return false;
    }
    let Some(rc) = GLOBAL.with(|g| g.borrow().as_ref().and_then(|w| w.upgrade())) else {
        return false;
    };
    let Ok(mut a) = rc.try_borrow_mut() else {
        return false;
    };
    // Plain keys belong to text fields and to the focused list.
    if plain {
        let focus = fapp::focus();
        let ours = focus
            .as_ref()
            .map(|f| {
                let p = f.as_widget_ptr() as usize;
                p == a.ui.files.wid.as_widget_ptr() as usize
                    || p == a.ui.playlist.wid.as_widget_ptr() as usize
                    || p == a.ui.tree.as_widget_ptr() as usize
                    || p == a.ui.win.as_widget_ptr() as usize
            })
            .unwrap_or(true);
        if !ours {
            return false;
        }
    }
    crate::app::handle_key(&mut a)
}

fn drop_files(a: &mut App, text: &str) {
    for line in text.lines() {
        let p = PathBuf::from(
            line.trim()
                .trim_start_matches("file://")
                .replace("%20", " "),
        );
        if p.is_file() {
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
            }
        } else if p.is_dir() {
            a.add_root(p);
        }
    }
}
