mod library;
mod logic;
mod player;
mod settings;

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fltk::{
    app,
    browser::HoldBrowser,
    button::Button,
    dialog::{FileDialog, FileDialogType},
    enums::{Align, Event, Key, Shortcut},
    frame::Frame,
    group::{Flex, Tile},
    prelude::*,
    tree::{Tree, TreeReason},
    window::Window,
};

use logic::mmss;
use player::Audio;
use settings::Settings;

pub const APP_NAME: &str = "1by1-rs";
const TICK: f64 = 0.25;
const VOL_STEP: f32 = 0.05;
/// Body area under the toolbar: x, y, w, h. Tree is 300 wide when shown.
const BODY: (i32, i32, i32, i32) = (4, 36, 892, 538);
const TREE_W: i32 = 300;

struct State {
    audio: Option<Audio>,
    settings: Settings,
    settings_path: Option<PathBuf>,
    dir: Option<PathBuf>,
    /// Files of `dir` in play order. Shuffled when shuffle is on.
    queue: Vec<PathBuf>,
    cur: usize,
    muted: bool,
    /// Tree pathname (`Music/Jazz`) to folder on disk.
    paths: HashMap<String, PathBuf>,
    tree: Tree,
    list: HoldBrowser,
    status: Frame,
    flags: Frame,
    win: Window,
    repeat_btn: Button,
    shuffle_btn: Button,
}

type Shared = Rc<RefCell<State>>;

impl State {
    fn say(&mut self, msg: impl AsRef<str>) {
        self.status.set_label(msg.as_ref());
    }

    fn cur_path(&self) -> Option<PathBuf> {
        self.queue.get(self.cur).cloned()
    }

    fn selected(&self) -> Option<usize> {
        let v = self.list.value();
        (v > 0 && (v as usize) <= self.queue.len()).then(|| v as usize - 1)
    }

    fn volume_now(&self) -> f32 {
        if self.muted { 0.0 } else { self.settings.volume }
    }

    fn refresh_buttons(&mut self) {
        self.repeat_btn.set_label(&format!("Repeat: {}", self.settings.repeat.label()));
        let shuffle = if self.settings.shuffle { "on" } else { "off" };
        self.shuffle_btn.set_label(&format!("Shuffle: {shuffle}"));
        self.refresh_flags();
    }

    /// Right side of the status bar. Short codes, as in the Readme.
    fn refresh_flags(&mut self) {
        let mut parts = vec![format!("REPEAT {}", self.settings.repeat.label().to_uppercase())];
        if self.settings.shuffle {
            parts.push("SHUF".into());
        }
        parts.push(format!("VOL {}%", (self.settings.volume * 100.0).round()));
        if self.muted {
            parts.push("MUTE".into());
        }
        self.flags.set_label(&parts.join("  "));
    }

    fn toggle_tree(&mut self) {
        let (x, y, w, h) = BODY;
        if self.tree.visible() {
            self.tree.hide();
            self.list.resize(x, y, w, h);
        } else {
            self.tree.show();
            self.list.resize(x + TREE_W, y, w - TREE_W, h);
        }
        self.list.redraw();
    }

    fn load_dir(&mut self, dir: PathBuf) {
        self.settings.last_dir = Some(dir.clone());
        self.dir = Some(dir);
        self.relist(None);
    }

    /// Rebuild queue and list from disk. `keep` stays current if still present.
    fn relist(&mut self, keep: Option<PathBuf>) {
        let Some(dir) = self.dir.clone() else { return };
        let mut files = library::audio_files(&dir);
        if self.settings.shuffle {
            logic::shuffle(&mut files, seed());
        }
        self.queue = files;
        self.list.clear();
        for f in &self.queue {
            self.list.add(&label(f));
        }
        self.cur = keep.and_then(|k| self.queue.iter().position(|p| *p == k)).unwrap_or(0);
        if !self.queue.is_empty() {
            self.list.select(self.cur as i32 + 1);
        }
    }

    /// Start `queue[idx]` at `seek`. Returns false and shows the error on failure.
    fn play_index(&mut self, idx: usize, seek: Duration) -> bool {
        let Some(path) = self.queue.get(idx).cloned() else { return false };
        let vol = self.volume_now();
        let Some(audio) = self.audio.as_mut() else {
            self.say("No audio output device.");
            return false;
        };
        match audio.play(&path, seek, vol) {
            Ok(()) => {
                self.cur = idx;
                self.list.select(idx as i32 + 1);
                self.settings.last_file = Some(path.clone());
                self.settings.last_pos = seek.as_secs_f64();
                self.win.set_label(&format!("{} - {APP_NAME}", name(&path)));
                self.say(format!("Playing: {}", name(&path)));
                true
            }
            Err(e) => {
                self.say(format!("Cannot play {}: {e}", name(&path)));
                false
            }
        }
    }

    /// Next track, skipping files that fail to open. Stops at end of list.
    fn next(&mut self) {
        let mut failed = 0;
        let hit = logic::advance_playable(self.queue.len(), self.cur, self.settings.repeat, |i| {
            let ok = self.play_index(i, Duration::ZERO);
            failed += usize::from(!ok);
            ok
        });
        match hit {
            Some(_) if failed > 0 => {
                let msg = format!("{} | skipped {failed} unplayable", self.status_text());
                self.say(msg);
            }
            Some(_) => {}
            None => self.end_of_list(),
        }
    }

    fn prev(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let idx = logic::prev_index(self.cur);
        self.play_index(idx, Duration::ZERO);
    }

    fn end_of_list(&mut self) {
        if let Some(a) = self.audio.as_mut() {
            a.stop();
        }
        self.settings.last_pos = 0.0;
        self.say("End of list.");
    }

    fn status_text(&self) -> String {
        self.settings.last_file.as_deref().map(name).unwrap_or_default()
    }

    /// Space. Playing stops. Stopped resumes the saved file at the saved position.
    fn toggle(&mut self) {
        if self.audio.as_ref().is_some_and(|a| a.is_playing()) {
            self.stop();
            return;
        }
        let Some(cur) = self.cur_path() else {
            self.say("No track. Pick a folder first.");
            return;
        };
        let seek = if self.settings.last_file.as_ref() == Some(&cur) {
            Duration::from_secs_f64(self.settings.last_pos)
        } else {
            Duration::ZERO
        };
        self.play_index(self.cur, seek);
    }

    fn stop(&mut self) {
        if let Some(a) = self.audio.as_mut() {
            if let Some(p) = a.pos() {
                self.settings.last_pos = p.as_secs_f64();
            }
            a.stop();
        }
        self.say("Stopped. Space resumes.");
    }

    fn set_volume_step(&mut self, up: bool) {
        let v = self.settings.volume + if up { VOL_STEP } else { -VOL_STEP };
        self.settings.volume = v.clamp(0.0, 1.0);
        self.apply_volume();
        self.refresh_buttons();
        self.say(format!("Volume {}%", (self.settings.volume * 100.0).round()));
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.apply_volume();
        self.refresh_buttons();
        self.say(if self.muted { "Muted" } else { "Unmuted" });
    }

    fn apply_volume(&mut self) {
        let v = self.volume_now();
        if let Some(a) = &self.audio {
            a.set_volume(v);
        }
    }

    fn cycle_repeat(&mut self) {
        self.settings.repeat = self.settings.repeat.cycle();
        self.refresh_buttons();
        self.say(format!("Repeat: {}", self.settings.repeat.label()));
    }

    fn toggle_shuffle(&mut self) {
        self.settings.shuffle = !self.settings.shuffle;
        let keep = self.cur_path();
        self.relist(keep);
        self.refresh_buttons();
        self.say(if self.settings.shuffle { "Shuffle on" } else { "Shuffle off" });
    }

    /// Del. Removes the selected file from the list. Removing the playing file plays its replacement.
    fn skip_selected(&mut self) {
        let Some(sel) = self.selected() else { return };
        let was_current = sel == self.cur;
        self.queue.remove(sel);
        self.list.remove(sel as i32 + 1);
        if sel < self.cur {
            self.cur -= 1;
        }
        if !was_current {
            return;
        }
        if self.queue.is_empty() {
            self.end_of_list();
        } else {
            self.play_index(sel.min(self.queue.len() - 1), Duration::ZERO);
        }
    }

    fn home(&mut self) {
        if !self.queue.is_empty() {
            self.list.select(self.cur as i32 + 1);
        }
    }

    fn tick(&mut self) {
        let Some(a) = self.audio.as_ref() else { return };
        if a.is_playing() {
            if let Some(p) = a.pos() {
                self.settings.last_pos = p.as_secs_f64();
                let msg = format!("{} | {}", name_of(self.cur_path()), mmss(p));
                self.say(msg);
            }
        } else if a.finished() {
            self.next();
        }
    }

    fn save(&self) {
        if let Some(p) = &self.settings_path {
            if let Err(e) = settings::save(p, &self.settings) {
                eprintln!("settings not saved: {e}");
            }
        }
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn name_of(p: Option<PathBuf>) -> String {
    p.as_deref().map(name).unwrap_or_default()
}

/// FLTK browser lines treat a leading `@` as a format code. Double it to show it literally.
fn label(p: &Path) -> String {
    name(p).replace('@', "@@")
}

/// Tree key for a root folder. Two roots with the same name get a suffix.
fn root_key(paths: &HashMap<String, PathBuf>, dir: &Path) -> String {
    let base = name(dir).replace('/', "_");
    let base = if base.is_empty() { "Root".to_string() } else { base };
    let mut key = base.clone();
    let mut n = 2;
    while paths.contains_key(&key) {
        key = format!("{base} ({n})");
        n += 1;
    }
    key
}

/// Adds `dir` and its subfolders. Full scan on add.
// ponytail: full recursive scan on add. Per-node lazy scan if big libraries stall.
fn add_node(tree: &mut Tree, paths: &mut HashMap<String, PathBuf>, key: String, dir: &Path) {
    let _ = tree.add(&key);
    paths.insert(key.clone(), dir.to_path_buf());
    for sub in library::subdirs(dir) {
        let child = format!("{key}/{}", name(&sub));
        add_node(tree, paths, child, &sub);
    }
}

/// Rebuild tree from `settings.roots`, then re-select `last_dir`. Does not fire callbacks.
fn build_tree(st: &mut State) {
    st.tree.clear();
    st.paths.clear();
    for root in st.settings.roots.clone() {
        if root.is_dir() {
            let key = root_key(&st.paths, &root);
            add_node(&mut st.tree, &mut st.paths, key, &root);
        }
    }
    let Some(dir) = st.settings.last_dir.clone() else { return };
    if let Some(key) = st.paths.iter().find(|(_, p)| **p == dir).map(|(k, _)| k.clone()) {
        let _ = st.tree.select(&key, false);
    }
}

impl State {
    fn add_root(&mut self, dir: PathBuf) {
        if self.settings.roots.contains(&dir) {
            return;
        }
        self.settings.roots.push(dir.clone());
        let key = root_key(&self.paths, &dir);
        add_node(&mut self.tree, &mut self.paths, key.clone(), &dir);
        let _ = self.tree.select(&key, false);
        self.load_dir(dir);
        self.save();
    }
}

/// Dropped text: a path or `file://` URI. A file means its folder.
fn drop_dir(line: &str) -> Option<PathBuf> {
    let p = PathBuf::from(line.trim().trim_start_matches("file://").replace("%20", " "));
    if p.is_dir() {
        Some(p)
    } else if p.is_file() {
        p.parent().map(Path::to_path_buf)
    } else {
        None
    }
}

fn pick_dir() -> Option<PathBuf> {
    let mut d = FileDialog::new(FileDialogType::BrowseDir);
    d.set_title("Add folder");
    d.show();
    let p = d.filename();
    (!p.as_os_str().is_empty()).then_some(p)
}

fn handle_key(st: &mut State) -> bool {
    let ctrl = app::event_state().contains(Shortcut::Ctrl);
    let key = app::event_key();
    let space = Key::from_char(' ');
    match (ctrl, key) {
        (false, k) if k == space => st.toggle(),
        (false, Key::Enter) => {
            if let Some(sel) = st.selected() {
                st.play_index(sel, Duration::ZERO);
            }
        }
        (false, Key::Pause) => st.toggle_mute(),
        (false, Key::F4) => st.toggle_tree(),
        (false, Key::Delete) => st.skip_selected(),
        (false, Key::F5) | (true, Key::End) => {
            build_tree(st);
            st.say("Rescanned.");
        }
        (true, Key::Up) => st.set_volume_step(true),
        (true, Key::Down) => st.set_volume_step(false),
        (true, Key::Left) => st.prev(),
        (true, Key::Right) => st.next(),
        (true, Key::Home) => st.home(),
        (true, k) if k == Key::from_char('h') => st.home(),
        (true, k) if k == Key::from_char('r') => st.cycle_repeat(),
        (true, k) if k == Key::from_char('o') => st.toggle_shuffle(),
        (true, k) if k == Key::from_char('q') => {
            st.save();
            app::quit();
        }
        _ => return false,
    }
    true
}

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut win = Window::default().with_size(900, 600).with_label(APP_NAME);

    let mut col = Flex::default_fill().column();
    col.set_margin(4);

    let bar = Flex::default().row();
    let prev_b = Button::default().with_label("|<");
    let play_b = Button::default().with_label("Play");
    let stop_b = Button::default().with_label("Stop");
    let next_b = Button::default().with_label(">|");
    let repeat_b = Button::default().with_label("Repeat: off");
    let shuffle_b = Button::default().with_label("Shuffle: off");
    let add_b = Button::default().with_label("Add folder");
    bar.end();
    col.fixed(&bar, 32);

    // Body: folder tree left, track list right, resizable split.
    // ponytail: fixed body geometry for the 900x600 window. Resize is not wired yet.
    let (bx, by, bw, bh) = BODY;
    let tile = Tile::new(bx, by, bw, bh, None);
    let tree = Tree::new(bx, by, TREE_W, bh, None);
    let list = HoldBrowser::new(bx + TREE_W, by, bw - TREE_W, bh, None);
    tile.end();

    // Status bar: message left, mode flags right.
    let mut sbar = Flex::default().row();
    let mut status = Frame::default().with_label("Ready");
    status.set_align(Align::Inside | Align::Left);
    let mut flags = Frame::default();
    flags.set_align(Align::Inside | Align::Right);
    sbar.end();
    sbar.fixed(&flags, 320);
    col.fixed(&sbar, 22);

    col.end();
    win.end();
    win.resizable(&col);

    let settings_path = settings::path();
    let settings = settings_path.as_deref().map(settings::load).unwrap_or_default();
    let audio = Audio::open();
    let mut state = State {
        audio: None,
        settings,
        settings_path,
        dir: None,
        queue: Vec::new(),
        cur: 0,
        muted: false,
        paths: HashMap::new(),
        tree: tree.clone(),
        list: list.clone(),
        status: status.clone(),
        flags: flags.clone(),
        win: win.clone(),
        repeat_btn: repeat_b.clone(),
        shuffle_btn: shuffle_b.clone(),
    };
    match audio {
        Ok(a) => state.audio = Some(a),
        Err(e) => state.say(format!("No audio output: {e}")),
    }
    build_tree(&mut state);
    state.refresh_buttons();
    if let Some(dir) = state.settings.last_dir.clone() {
        state.load_dir(dir);
        if let Some(file) = state.settings.last_file.clone() {
            state.relist(Some(file.clone()));
            let pos = Duration::from_secs_f64(state.settings.last_pos);
            let msg = format!("Ready: {} at {}. Space resumes.", name(&file), mmss(pos));
            state.say(msg);
        }
    }

    let st: Shared = Rc::new(RefCell::new(state));

    let s = st.clone();
    let mut tree = tree;
    tree.set_callback(move |t| {
        if !matches!(t.callback_reason(), TreeReason::Selected | TreeReason::Reselected) {
            return;
        }
        let Some(item) = t.callback_item() else { return };
        let Ok(key) = t.item_pathname(&item) else { return };
        let mut st = s.borrow_mut();
        if let Some(dir) = st.paths.get(&key).cloned() {
            st.load_dir(dir);
        }
    });

    let s = st.clone();
    let mut list = list;
    list.handle(move |_, ev| {
        if ev == Event::Push && app::event_clicks() {
            let mut st = s.borrow_mut();
            if let Some(sel) = st.selected() {
                st.play_index(sel, Duration::ZERO);
            }
        }
        false
    });

    let s = st.clone();
    let mut b = prev_b;
    b.set_callback(move |_| s.borrow_mut().prev());
    let s = st.clone();
    let mut b = next_b;
    b.set_callback(move |_| s.borrow_mut().next());
    let s = st.clone();
    let mut b = play_b;
    b.set_callback(move |_| s.borrow_mut().toggle());
    let s = st.clone();
    let mut b = stop_b;
    b.set_callback(move |_| s.borrow_mut().stop());
    let s = st.clone();
    let mut b = repeat_b;
    b.set_callback(move |_| s.borrow_mut().cycle_repeat());
    let s = st.clone();
    let mut b = shuffle_b;
    b.set_callback(move |_| s.borrow_mut().toggle_shuffle());
    let s = st.clone();
    let mut b = add_b;
    // Dialog runs its own event loop. Do not hold the borrow across it.
    b.set_callback(move |_| {
        if let Some(dir) = pick_dir() {
            s.borrow_mut().add_root(dir);
        }
    });

    let s = st.clone();
    win.handle(move |_, ev| match ev {
        Event::DndEnter | Event::DndDrag | Event::DndRelease => true,
        Event::Paste => {
            let text = app::event_text();
            let mut st = s.borrow_mut();
            for dir in text.lines().filter_map(drop_dir) {
                st.add_root(dir);
            }
            true
        }
        Event::KeyDown => handle_key(&mut s.borrow_mut()),
        _ => false,
    });

    let s = st.clone();
    win.set_callback(move |_| {
        s.borrow().save();
        app::quit();
    });

    let s = st.clone();
    app::add_timeout3(TICK, move |h| {
        s.borrow_mut().tick();
        app::repeat_timeout3(TICK, h);
    });

    win.show();
    app.run().unwrap();
}
