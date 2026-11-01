#![allow(dead_code)]

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use fltk::{
    app,
    draw::{
        draw_rect_fill, draw_text2, set_draw_color, set_font, set_line_style, LineStyle,
    },
    enums::{Align, Color, Event, Font, FrameType},
    misc::Tooltip,
    prelude::*,
    widget::Widget,
};
use std::ffi::{CStr, CString};

use crate::{icons, theme::Palette};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cmd {
    Menu,
    Search,
    Explore,
    Intro,
    App1,
    App2,
    App3,
    Http,
    Play,
    StopResume,
    Pause,
    Prev,
    Next,
    PrevDir,
    NextDir,
    Repeat,
    Shuffle,
    Settings,
    Compact,
    Tree,
    TrackNext,
    Skip,
    Playlist,
    Reload,
    Home,
    Scan,
    Bookmarks,
    Goto,
    Find,
    FindDrive,
    QuitNext,
    BigView,
    Log,
    Picture,
    Mute,
    Info,
    TrackHistory,
    FolderHistory,
}

#[derive(Clone, Copy)]
struct Btn {
    cmd: Cmd,
    icon: icons::Icon,
    tip: &'static str,
    gap_before: bool,
}

const fn b(cmd: Cmd, icon: icons::Icon, tip: &'static str, gap_before: bool) -> Btn {
    Btn {
        cmd,
        icon,
        tip,
        gap_before,
    }
}

/// Toolbar groups: menu | find and browse | transport | list tools | view.
const BUTTONS: [Btn; 25] = [
    b(Cmd::Menu, icons::Icon::Menu, "Main menu  [F1 or right button]", false),
    b(Cmd::Search, icons::Icon::Search, "Find files  [F3]", true),
    b(Cmd::Explore, icons::Icon::Explore, "Explore folder  [Ctrl+X]", false),
    b(Cmd::Intro, icons::Icon::Intro, "Intro scan  [Ctrl+N]", false),
    b(Cmd::Play, icons::Icon::Play, "Play  [Return]", true),
    b(
        Cmd::StopResume,
        icons::Icon::Stop,
        "Stop / Resume  [Space]",
        false,
    ),
    b(Cmd::Pause, icons::Icon::Pause, "Pause / Resume  [Pause]", false),
    b(Cmd::Prev, icons::Icon::Prev, "Previous track  [Ctrl+Left]", false),
    b(Cmd::Next, icons::Icon::Next, "Next track  [Ctrl+Right]", false),
    b(
        Cmd::PrevDir,
        icons::Icon::PrevDir,
        "Previous folder  [Shift+Left]",
        false,
    ),
    b(
        Cmd::NextDir,
        icons::Icon::NextDir,
        "Next folder  [Shift+Right]",
        false,
    ),
    b(Cmd::Repeat, icons::Icon::RepeatOff, "Repeat mode  [Ctrl+R]", true),
    b(Cmd::Shuffle, icons::Icon::Shuffle, "Shuffle list  [Ctrl+P]", false),
    b(Cmd::Playlist, icons::Icon::Playlist, "Playlist view  [Shift+F5]", false),
    b(Cmd::TrackNext, icons::Icon::TrackNext, "Track = next  [Ins]", false),
    b(
        Cmd::Skip,
        icons::Icon::Skip,
        "Skip: remove track from list  [Del]",
        false,
    ),
    b(Cmd::Reload, icons::Icon::Reload, "Reload folder  [F5]", true),
    b(Cmd::Home, icons::Icon::Home, "Go to current track  [Ctrl+H]", false),
    b(Cmd::Scan, icons::Icon::Scan, "Scan times and info  [Shift+F4]", false),
    b(Cmd::Goto, icons::Icon::Goto, "Go to folder  [Ctrl+Shift+G]", false),
    b(Cmd::BigView, icons::Icon::BigView, "Big title view  [Ctrl+B]", true),
    b(Cmd::Compact, icons::Icon::Compact, "Compact view  [F7]", false),
    b(Cmd::Tree, icons::Icon::Tree, "Folder tree on/off  [F4]", false),
    b(Cmd::Info, icons::Icon::Info, "Track info  [Ctrl+I]", false),
    b(Cmd::Settings, icons::Icon::Settings, "Settings  [F6]", true),
];

pub struct ToolbarModel {
    pub pal: Palette,
    pub active: HashSet<Cmd>,
    pub hover: Option<Cmd>,
    pub pressed: Option<Cmd>,
    pub size: i32,
    pub show_tooltips: bool,
    tips: Vec<(Cmd, &'static CStr)>,
}

impl ToolbarModel {
    pub fn new(pal: Palette) -> Self {
        Self {
            pal,
            active: HashSet::new(),
            hover: None,
            pressed: None,
            size: 28,
            show_tooltips: true,
            tips: BUTTONS
                .iter()
                .map(|b| {
                    let c = CString::new(b.tip).unwrap();
                    (b.cmd, Box::leak(c.into_boxed_c_str()) as &'static CStr)
                })
                .collect(),
        }
    }

    fn tip(&self, cmd: Cmd) -> &'static CStr {
        self.tips
            .iter()
            .find(|(c, _)| *c == cmd)
            .map(|(_, t)| *t)
            .unwrap_or(c"")
    }

    pub fn set_active(&mut self, cmd: Cmd, on: bool) {
        if on {
            self.active.insert(cmd);
        } else {
            self.active.remove(&cmd);
        }
    }
}

type CmdSink = Rc<RefCell<Option<Box<dyn FnMut(Cmd)>>>>;
type HintSink = Rc<RefCell<Option<Box<dyn FnMut(Option<String>)>>>>;

#[derive(Clone)]
pub struct Toolbar {
    pub wid: Widget,
    pub model: Rc<RefCell<ToolbarModel>>,
    sink: CmdSink,
    hint: HintSink,
}

impl Toolbar {
    pub fn new(x: i32, y: i32, w: i32, h: i32, model: Rc<RefCell<ToolbarModel>>) -> Self {
        let mut wid = Widget::new(x, y, w, h, "");
        wid.set_frame(FrameType::FlatBox);
        wid.set_color(model.borrow().pal.window);
        let sink: CmdSink = Rc::new(RefCell::new(None));
        let hint: HintSink = Rc::new(RefCell::new(None));

        let m = model.clone();
        wid.draw(move |w| draw_toolbar(w, &m.borrow()));

        let m = model.clone();
        let s = sink.clone();
        let h = hint.clone();
        wid.handle(move |w, ev| handle(w, ev, &m, &s, &h));
        Self {
            wid,
            model,
            sink,
            hint,
        }
    }

    pub fn on_cmd<F: FnMut(Cmd) + 'static>(&self, f: F) {
        *self.sink.borrow_mut() = Some(Box::new(f));
    }

    /// Called with the hovered button hint, or None when the pointer leaves.
    pub fn on_hover<F: FnMut(Option<String>) + 'static>(&self, f: F) {
        *self.hint.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_theme(&mut self, pal: Palette) {
        self.model.borrow_mut().pal = pal;
        self.wid.set_color(pal.window);
        self.wid.redraw();
    }

    pub fn redraw(&mut self) {
        self.wid.redraw();
    }
}

/// Button rectangles in widget-local coordinates.
fn layout(width: i32) -> Vec<(Btn, i32, i32)> {
    let size = 28;
    let mut x = 2;
    let mut out: Vec<(Btn, i32, i32)> = Vec::new();
    for btn in BUTTONS {
        // The native macOS menu bar replaces the hamburger.
        if cfg!(target_os = "macos") && btn.cmd == Cmd::Menu {
            continue;
        }
        if btn.gap_before && !out.is_empty() {
            x += 11;
        }
        if x + size > width {
            break;
        }
        out.push((btn, x, size));
        x += size + 2;
    }
    out
}

fn shade(c: Color, delta: i32) -> Color {
    let (r, g, b) = c.to_rgb();
    let f = |v: u8| (v as i32 + delta).clamp(0, 255) as u8;
    Color::from_rgb(f(r), f(g), f(b))
}

/// Win95-style bevel. Light edges on the top left, dark on the bottom right.
fn draw_toolbar(w: &mut Widget, m: &ToolbarModel) {
    let pal = m.pal;
    let (x, y, ww, hh) = (w.x(), w.y(), w.w(), w.h());
    draw_rect_fill(x, y, ww, hh, pal.window);

    let h = m.size;
    let by = y + (hh - h) / 2;
    for (i, (btn, bx, _size)) in layout(ww).into_iter().enumerate() {
        if btn.gap_before && i > 0 {
            let sx = x + bx - 7;
            draw_rect_fill(sx, y + (hh - 18) / 2, 1, 18, shade(pal.window, -28));
            draw_rect_fill(sx + 1, y + (hh - 18) / 2, 1, 18, shade(pal.window, 35));
        }
    }

    for (btn, bx, size) in layout(ww) {
        let rx = x + bx;
        let pressed = m.pressed == Some(btn.cmd);
        let hover = m.hover == Some(btn.cmd);
        let active = m.active.contains(&btn.cmd);
        if pressed {
            draw_rect_fill(rx, by, size, h, pal.button_hi);
        } else if hover {
            draw_rect_fill(
                rx,
                by,
                size,
                h,
                shade(pal.window, if pal.dark { 20 } else { -10 }),
            );
        }
        let oy = if pressed { 1 } else { 0 };
        let fg = pal.text;
        let (ix, iy, iw, ih) = (rx + 5, by + 5 + oy, size - 10, h - 10);
        if !crate::iconpack::draw(btn.icon, ix, iy, iw, ih, pal.dark) {
            icons::draw(btn.icon, ix, iy, iw, ih, fg);
        }
        if active {
            draw_rect_fill(rx + 5, by + h - 5, size - 10, 2, pal.list_sel);
        }
    }
}

fn handle(
    w: &mut Widget,
    ev: Event,
    m: &Rc<RefCell<ToolbarModel>>,
    sink: &CmdSink,
    hint: &HintSink,
) -> bool {
    let mx = app::event_x() - w.x();
    let mut model = m.borrow_mut();
    let width = w.w();
    let hit = move |mx: i32| {
        layout(width)
            .into_iter()
            .find(|(_, bx, s)| mx >= *bx && mx < bx + s)
    };

    match ev {
        Event::Enter | Event::Move => {
            let h = hit(mx);
            let cmd = h.map(|(b, _, _)| b.cmd);
            let tips = model.show_tooltips;
            let changed = model.hover != cmd;
            if changed {
                model.hover = cmd;
                let text = h.map(|(b, _, _)| b.tip.to_string());
                let tip = h.map(|(b, _, _)| model.tip(b.cmd));
                drop(model);
                w.redraw();
                if let Some(f) = hint.borrow_mut().as_mut() {
                    f(text);
                }
                // FLTK captures the tooltip when the pointer enters, so dynamic
                // hints must be pushed through enter_area explicitly.
                if let Some(tip) = tip {
                    Tooltip::enter_area(w, 0, 0, w.w(), w.h(), tip);
                } else {
                    Tooltip::enter_area(w, 0, 0, 0, 0, c"");
                }
            }
            if tips {
                if let Some((b, _, _)) = h {
                    w.set_tooltip(b.tip);
                } else {
                    w.set_tooltip("");
                }
            }
            false
        }
        Event::Leave => {
            model.hover = None;
            model.pressed = None;
            drop(model);
            w.redraw();
            if let Some(f) = hint.borrow_mut().as_mut() {
                f(None);
            }
            Tooltip::enter_area(w, 0, 0, 0, 0, c"");
            false
        }
        Event::Push => {
            if let Some((b, _, _)) = hit(mx) {
                model.pressed = Some(b.cmd);
                drop(model);
                w.redraw();
                return true;
            }
            false
        }
        Event::Released => {
            let pressed = model.pressed.take();
            drop(model);
            w.redraw();
            if let Some(cmd) = pressed {
                if let Some((b, _, _)) = hit(mx) {
                    if b.cmd == cmd {
                        if let Some(f) = sink.borrow_mut().as_mut() {
                            f(cmd);
                        }
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// A flat slider drawn by the app: position or volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderKind {
    Position,
    Volume,
}

pub struct SliderModel {
    pub kind: SliderKind,
    pub value: f64,
    pub pal: Palette,
    pub dragging: bool,
    pub enabled: bool,
}

pub type SliderSink = Rc<RefCell<Option<Box<dyn FnMut(f64)>>>>;

#[derive(Clone)]
pub struct FlatSlider {
    pub wid: Widget,
    pub model: Rc<RefCell<SliderModel>>,
    sink: SliderSink,
}

impl FlatSlider {
    pub fn new(x: i32, y: i32, w: i32, h: i32, model: Rc<RefCell<SliderModel>>) -> Self {
        let mut wid = Widget::new(x, y, w, h, "");
        wid.set_frame(FrameType::FlatBox);
        let hint = match model.borrow().kind {
            SliderKind::Position => "Playback position  [Left / Right]",
            SliderKind::Volume => "Volume  [Cmd+Up / Cmd+Down]",
        };
        wid.set_tooltip(hint);
        let sink: SliderSink = Rc::new(RefCell::new(None));

        let m = model.clone();
        wid.draw(move |w| {
            let m = m.borrow();
            let (x, y, ww, hh) = (w.x(), w.y(), w.w(), w.h());
            let pal = m.pal;
            draw_rect_fill(x, y, ww, hh, pal.window);
            let cy = y + hh / 2;
            let left = x + 8;
            let right = x + ww - 8;
            if right <= left {
                return;
            }
            // Track.
            draw_rect_fill(left, cy - 2, right - left, 4, pal.slider);
            let v = m.value.clamp(0.0, 1.0);
            let filled = ((right - left) as f64 * v) as i32;
            let accent = if m.enabled { pal.list_sel } else { pal.slider };
            draw_rect_fill(left, cy - 2, filled, 4, accent);
            // Round knob, drawn with the native filled-circle primitive so the
            // edges are anti-aliased by the platform.
            let kx = left + filled;
            let r = if m.kind == SliderKind::Volume { 7 } else { 6 };
            fltk::draw::set_draw_color(pal.slider_knob);
            fltk::draw::draw_pie(kx - r, cy - r, r * 2, r * 2, 0.0, 360.0);
            set_draw_color(pal.dim);
            set_line_style(LineStyle::Solid, 1);
            fltk::draw::draw_arc(kx - r, cy - r, r * 2, r * 2, 0.0, 360.0);
        });

        let m = model.clone();
        let s = sink.clone();
        wid.handle(move |w, ev| {
            let mut m = m.borrow_mut();
            let mx = app::event_x() - w.x();
            let frac = ((mx - 8) as f64 / (w.w() - 16).max(1) as f64).clamp(0.0, 1.0);
            match ev {
                Event::Push => {
                    if !m.enabled {
                        return true;
                    }
                    m.dragging = true;
                    m.value = frac;
                    let v = frac;
                    drop(m);
                    w.redraw();
                    if let Some(f) = s.borrow_mut().as_mut() {
                        f(v);
                    }
                    true
                }
                Event::Drag if m.dragging => {
                    m.value = frac;
                    let v = frac;
                    drop(m);
                    w.redraw();
                    if let Some(f) = s.borrow_mut().as_mut() {
                        f(v);
                    }
                    true
                }
                Event::Released => {
                    m.dragging = false;
                    false
                }
                _ => false,
            }
        });
        Self { wid, model, sink }
    }

    pub fn on_change<F: FnMut(f64) + 'static>(&self, f: F) {
        *self.sink.borrow_mut() = Some(Box::new(f));
    }

    pub fn set(&mut self, value: f64) {
        self.model.borrow_mut().value = value.clamp(0.0, 1.0);
        self.wid.redraw();
    }

    pub fn resize(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.wid.resize(x, y, w, h);
    }

    pub fn set_theme(&mut self, pal: Palette) {
        self.model.borrow_mut().pal = pal;
        self.wid.redraw();
    }
}

fn circle_fill(cx: i32, cy: i32, r: i32, color: Color) {
    for dy in -r..=r {
        let span = ((r * r - dy * dy) as f64).sqrt() as i32;
        draw_rect_fill(cx - span, cy + dy, span * 2 + 1, 1, color);
    }
}

fn ring(cx: i32, cy: i32, r: i32, color: Color) {
    set_draw_color(color);
    set_line_style(LineStyle::Solid, 1);
    let mut prev = (cx + r, cy);
    for i in 1..=28 {
        let a = i as f64 * std::f64::consts::TAU / 28.0;
        let p = (
            (cx as f64 + r as f64 * a.cos()) as i32,
            (cy as f64 + r as f64 * a.sin()) as i32,
        );
        fltk::draw::draw_line(prev.0, prev.1, p.0, p.1);
        prev = p;
    }
}

/// Big title view: large text, theme colours, layout from settings.
pub struct BigModel {
    pub pal: Palette,
    pub title: String,
    pub sub: String,
    pub info: String,
    pub time: String,
    pub lines: u8,
    pub padding: i32,
    pub show_icon: bool,
}

impl BigModel {
    pub fn new(pal: Palette) -> Self {
        Self {
            pal,
            title: "No track".into(),
            sub: String::new(),
            info: String::new(),
            time: String::new(),
            lines: 2,
            padding: 16,
            show_icon: true,
        }
    }
}

#[derive(Clone)]
pub struct BigView {
    pub wid: Widget,
    pub model: Rc<RefCell<BigModel>>,
}

impl BigView {
    pub fn new(x: i32, y: i32, w: i32, h: i32, model: Rc<RefCell<BigModel>>) -> Self {
        let mut wid = Widget::new(x, y, w, h, "");
        wid.set_frame(FrameType::FlatBox);
        let m = model.clone();
        wid.draw(move |w| {
            let m = m.borrow();
            let pal = m.pal;
            let (x, y, ww, hh) = (w.x(), w.y(), w.w(), w.h());
            draw_rect_fill(x, y, ww, hh, pal.big_bg);
            set_draw_color(pal.grid);
            set_line_style(LineStyle::Solid, 1);
            fltk::draw::draw_rect(x, y, ww, hh);

            let pad = m.padding.max(4);
            let inner_w = (ww - pad * 2).max(10);
            let per = ((hh - pad * 2) / m.lines.max(1) as i32).max(14);
            let mut title_px = (per * 6 / 10).clamp(14, 120);
            set_font(Font::HelveticaBold, title_px);
            let (tw, _) = fltk::draw::measure(&m.title, false);
            if tw > inner_w && tw > 0 {
                title_px = ((title_px as f64 * inner_w as f64 / tw as f64) as i32).clamp(11, title_px);
                set_font(Font::HelveticaBold, title_px);
            }
            set_draw_color(pal.big_text);
            draw_text2(
                &m.title,
                x + pad,
                y + pad,
                inner_w,
                title_px + 8,
                Align::Left | Align::Inside | Align::Clip,
            );

            let small_px = (title_px / 3).clamp(10, 34);
            let mut cy = y + pad + title_px + 10;
            set_font(Font::Helvetica, small_px);
            set_draw_color(pal.big_dim);
            if m.lines >= 3 && !m.sub.is_empty() {
                draw_text2(&m.sub, x + pad, cy, inner_w, small_px + 4, Align::Left | Align::Inside);
                cy += small_px + 6;
            }
            if m.lines >= 3 && !m.info.is_empty() {
                draw_text2(&m.info, x + pad, cy, inner_w, small_px + 4, Align::Left | Align::Inside);
            }

            if !m.time.is_empty() {
                let tpx = (small_px * 3 / 2).clamp(11, 42);
                set_font(Font::HelveticaBold, tpx);
                set_draw_color(pal.big_text);
                draw_text2(
                    &m.time,
                    x + pad,
                    y + hh - pad - tpx - 2,
                    inner_w,
                    tpx + 4,
                    Align::Right | Align::Inside,
                );
            }
        });
        Self { wid, model }
    }

    pub fn set_theme(&mut self, pal: Palette) {
        self.model.borrow_mut().pal = pal;
        self.wid.redraw();
    }

    pub fn resize(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.wid.resize(x, y, w, h);
    }

    pub fn redraw(&mut self) {
        self.wid.redraw();
    }
}
