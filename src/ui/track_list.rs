#![allow(dead_code)]

use std::{
    cell::RefCell,
    collections::BTreeSet,
    ffi::{CStr, CString},
    rc::Rc,
};

use fltk::{
    app,
    draw::{draw_box, draw_rect_fill, draw_text2, pop_clip, push_clip, set_draw_color, set_font,
        set_line_style, LineStyle},
    enums::{Align, Color, Event, Font, FrameType, Key, Shortcut},
    prelude::*,
    widget::Widget,
};

use crate::{
    library::Track,
    logic::{clock, date_parts, date_text, month_tint, SortKey},
    theme::Palette,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Col {
    Name,
    Length,
    Size,
    Type,
    Modified,
    Created,
    Info,
}

impl Col {
    pub fn label(self) -> &'static str {
        match self {
            Col::Name => "Name",
            Col::Length => "Length",
            Col::Size => "Size",
            Col::Type => "Type",
            Col::Modified => "Modified",
            Col::Created => "Created",
            Col::Info => "Info",
        }
    }

    pub fn key(self) -> Option<SortKey> {
        match self {
            Col::Name => Some(SortKey::Name),
            Col::Length => Some(SortKey::Length),
            Col::Size => Some(SortKey::Size),
            Col::Type => Some(SortKey::Type),
            Col::Modified => Some(SortKey::Modified),
            Col::Created => Some(SortKey::Created),
            Col::Info => None,
        }
    }

    fn align(self) -> Align {
        match self {
            Col::Length | Col::Size => Align::Right | Align::Inside | Align::Clip,
            _ => Align::Left | Align::Inside | Align::Clip,
        }
    }

    fn width(self, width: i32, name_w: i32) -> i32 {
        let _ = width;
        match self {
            Col::Name => name_w,
            Col::Length => 58,
            Col::Size => 104,
            Col::Type => 46,
            Col::Modified | Col::Created => 92,
            Col::Info => name_w,
        }
    }
}

/// What the list asks the app to do. Emitted from mouse and key handling.
#[derive(Clone, Debug)]
pub enum ListEvent {
    /// Row focused or clicked.
    Select(usize),
    /// Return or double click.
    Activate(usize),
    /// Right click at root coordinates.
    Context(usize, i32, i32),
    /// Column header click, given the column index.
    Header(usize),
    /// Shift+Up/Down on a row.
    Move(i32),
    /// Ctrl+A.
    SelectAll,
}

pub type Sink = Rc<RefCell<Option<Box<dyn FnMut(ListEvent)>>>>;

pub struct ListModel {
    pub tracks: Vec<Track>,
    pub cur: Option<usize>,
    pub focus: usize,
    pub sel: BTreeSet<usize>,
    pub top: i32,
    pub cols: Vec<Col>,
    pub pal: Palette,
    pub rowh: i32,
    pub header_h: i32,
    pub grid: bool,
    pub full_row: bool,
    pub show_header: bool,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub month_colors: bool,
    pub empty_text: String,
    pub scrollbar: bool,
    /// Scrollbar thumb drag state.
    pub scroll_drag: bool,
    pub scroll_grab: i32,
    /// Row whose tooltip is currently pushed to FLTK.
    pub tip_row: Option<usize>,
}

impl ListModel {
    pub fn new(pal: Palette) -> Self {
        Self {
            tracks: Vec::new(),
            cur: None,
            focus: 0,
            sel: BTreeSet::new(),
            top: 0,
            cols: vec![Col::Name, Col::Length, Col::Size, Col::Type, Col::Modified],
            pal,
            rowh: 20,
            header_h: 20,
            grid: false,
            full_row: true,
            show_header: true,
            sort: SortKey::Name,
            sort_desc: false,
            month_colors: false,
            empty_text: "No tracks".into(),
            scrollbar: true,
            scroll_drag: false,
            scroll_grab: 0,
            tip_row: None,
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.tracks = tracks;
        self.sel.clear();
        self.focus = 0;
        self.top = 0;
        self.cur = None;
    }

    pub fn clear_selection(&mut self) {
        self.sel.clear();
    }

    pub fn selected_rows(&self) -> Vec<usize> {
        if self.tracks.is_empty() {
            return Vec::new();
        }
        if self.sel.is_empty() {
            vec![self.focus.min(self.tracks.len() - 1)]
        } else {
            self.sel.iter().copied().filter(|r| *r < self.tracks.len()).collect()
        }
    }

    pub fn text(&self, row: usize, col: Col) -> String {
        let Some(t) = self.tracks.get(row) else {
            return String::new();
        };
        match col {
            Col::Name => t.name.clone(),
            Col::Length => t.duration().map(clock).unwrap_or_default(),
            Col::Size => match t.duration() {
                Some(d) => format!("{}  {}", clock(d), t.size_text()),
                None => t.size_text(),
            },
            Col::Type => t.type_text(),
            Col::Modified => t.modified.map(date_text).unwrap_or_default(),
            Col::Created => t.created.map(date_text).unwrap_or_default(),
            Col::Info => t.info_text(),
        }
    }

    fn row_month(&self, row: usize) -> u32 {
        self.tracks
            .get(row)
            .and_then(|t| t.modified)
            .map(|m| date_parts(m).1)
            .unwrap_or(0)
    }

    pub fn ensure_visible(&mut self, row: usize, height: i32) {
        let visible = ((height - self.header_h) / self.rowh).max(1);
        if (row as i32) < self.top {
            self.top = row as i32;
        } else if (row as i32) >= self.top + visible {
            self.top = row as i32 - visible + 1;
        }
    }

    fn visible_rows(&self, height: i32) -> i32 {
        ((height - self.header_h) / self.rowh).max(1)
    }
}

#[derive(Clone)]
pub struct TList {
    pub wid: Widget,
    pub model: Rc<RefCell<ListModel>>,
    sink: Sink,
}

impl TList {
    pub fn new(x: i32, y: i32, w: i32, h: i32, model: Rc<RefCell<ListModel>>) -> Self {
        let mut wid = Widget::new(x, y, w, h, "");
        wid.set_frame(fltk::enums::FrameType::FlatBox);
        wid.set_color(model.borrow().pal.list_bg);
        let sink: Sink = Rc::new(RefCell::new(None));

        let m = model.clone();
        wid.draw(move |w| {
            let mut model = m.borrow_mut();
            draw_list(w, &mut model);
        });

        let m = model.clone();
        let s = sink.clone();
        wid.handle(move |w, ev| handle(w, ev, &m, &s));
        Self { wid, model, sink }
    }

    pub fn on_event<F: FnMut(ListEvent) + 'static>(&self, f: F) {
        *self.sink.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_theme(&mut self, pal: Palette) {
        self.model.borrow_mut().pal = pal;
        self.wid.set_color(pal.list_bg);
        self.wid.redraw();
    }

    pub fn redraw(&mut self) {
        self.wid.redraw();
    }

    pub fn resize(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.wid.resize(x, y, w, h);
    }
}

fn handle(w: &mut Widget, ev: Event, m: &Rc<RefCell<ListModel>>, sink: &Sink) -> bool {
    let emit = |e: ListEvent| {
        if let Ok(mut slot) = sink.try_borrow_mut() {
            if let Some(f) = slot.as_mut() {
                f(e);
            }
        }
    };
    let (mx, my) = (app::event_x() - w.x(), app::event_y() - w.y());
    let (ww, hh) = (w.w(), w.h());
    let dy = app::event_dy_value();
    let mut model = m.borrow_mut();
    let header = if model.show_header { model.header_h } else { 0 };
    let body_h = (hh - header).max(1);
    let visible = model.visible_rows(hh);
    let len = model.tracks.len();

    match ev {
        Event::Focus | Event::Unfocus => true,
        Event::Move => {
            let row = if my < header {
                None
            } else {
                let r = model.top + (my - header) / model.rowh;
                (r >= 0 && (r as usize) < len).then_some(r as usize)
            };
            if row != model.tip_row {
                model.tip_row = row;
                if let Some(row) = row {
                    let t = &model.tracks[row];
                    let mut hint = t.name.clone();
                    if let Some(d) = t.duration() {
                        hint.push_str(&format!("   {}", clock(d)));
                    }
                    hint.push_str(&format!("   {}", t.size_text()));
                    if !t.ext.is_empty() {
                        hint.push_str(&format!("   {}", t.ext.to_uppercase()));
                    }
                    hint.push_str("   |   double click to play, right click for the menu");
                    let text = CString::new(hint).unwrap();
                    let tip: &'static CStr =
                        Box::leak(text.into_boxed_c_str());
                    w.set_tooltip(&t.name);
                    fltk::misc::Tooltip::enter_area(w, 0, 0, w.w(), w.h(), tip);
                } else {
                    w.set_tooltip("");
                    fltk::misc::Tooltip::enter_area(w, 0, 0, 0, 0, c"");
                }
            }
            false
        }
        Event::Leave => {
            model.tip_row = None;
            w.set_tooltip("");
            fltk::misc::Tooltip::enter_area(w, 0, 0, 0, 0, c"");
            false
        }
        Event::MouseWheel => {
            drop(model);
            let mut model = m.borrow_mut();
            model.top = (model.top - dy).clamp(0, (len as i32 - visible).max(0));
            drop(model);
            w.redraw();
            true
        }
        Event::Push => {
            let _ = w.take_focus();
            if my < header {
                let (col, _) = col_at(&model, mx, ww);
                if let Some(c) = col {
                    let idx = model.cols.iter().position(|k| *k == c).unwrap_or(0);
                    drop(model);
                    emit(ListEvent::Header(idx));
                }
                return true;
            }
            if model.scrollbar && len as i32 > visible && mx >= ww - SCROLL_W {
                let (arrow, track) = scroll_region(body_h);
                let y = my - header;
                if y < arrow {
                    model.top = (model.top - 1).max(0);
                    drop(model);
                    w.redraw();
                    return true;
                }
                if y >= arrow + track {
                    let max = (len as i32 - visible).max(0);
                    model.top = (model.top + 1).min(max);
                    drop(model);
                    w.redraw();
                    return true;
                }
                let (thumb, off) = scroll_thumb(track, len as i32, visible, model.top);
                let y = y - arrow;
                let grab = if y >= off && y < off + thumb {
                    y - off
                } else {
                    thumb / 2
                };
                model.scroll_grab = grab;
                model.scroll_drag = true;
                let max = (len as i32 - visible).max(0);
                model.top = scroll_top(y - grab, track, thumb, max);
                drop(model);
                w.redraw();
                return true;
            }
            let row = model.top + (my - header) / model.rowh;
            if row < 0 || row as usize >= len {
                return true;
            }
            let row = row as usize;
            if app::event_button() == 3 {
                if !model.sel.contains(&row) {
                    model.sel.clear();
                    model.sel.insert(row);
                }
                model.focus = row;
                drop(model);
                w.redraw();
                let (rx, ry) = (app::event_x_root(), app::event_y_root());
                emit(ListEvent::Context(row, rx, ry));
                return true;
            }
            let shift = app::event_state().contains(Shortcut::Shift);
            let ctrl = app::event_state().contains(Shortcut::Command);
            if shift {
                let anchor = model.sel.iter().next().copied().unwrap_or(model.focus);
                let (a, b) = if anchor <= row { (anchor, row) } else { (row, anchor) };
                model.sel = (a..=b).collect();
            } else if ctrl {
                if !model.sel.remove(&row) {
                    model.sel.insert(row);
                }
            } else {
                model.sel.clear();
                model.sel.insert(row);
            }
            model.focus = row;
            model.ensure_visible(row, hh);
            let dbl = app::event_clicks();
            drop(model);
            w.redraw();
            emit(ListEvent::Select(row));
            if dbl {
                emit(ListEvent::Activate(row));
            }
            true
        }
        Event::Drag => {
            if model.scroll_drag {
                let (arrow, track) = scroll_region(body_h);
                let (thumb, _) = scroll_thumb(track, len as i32, visible, model.top);
                let max = (len as i32 - visible).max(0);
                model.top = scroll_top(my - header - arrow - model.scroll_grab, track, thumb, max);
                drop(model);
                w.redraw();
                return true;
            }
            if mx > ww - SCROLL_W {
                let (arrow, track) = scroll_region(body_h);
                let frac = ((my - header - arrow) as f64 / track as f64).clamp(0.0, 1.0);
                let max = (len as i32 - visible).max(0);
                model.top = (frac * max as f64).round() as i32;
                drop(model);
                w.redraw();
                return true;
            }
            false
        }
        Event::Released => {
            if model.scroll_drag {
                model.scroll_drag = false;
                return true;
            }
            false
        }
        Event::KeyDown => {
            if len == 0 {
                return false;
            }
            let key = app::event_key();
            let shift = app::event_state().contains(Shortcut::Shift);
            let ctrl = app::event_state().contains(Shortcut::Command);
            if ctrl && !(key == Key::from_char('a')) {
                return false;
            }
            let mut row = model.focus.min(len - 1);
            match key {
                Key::Up if shift => {
                    drop(model);
                    emit(ListEvent::Move(-1));
                    return true;
                }
                Key::Down if shift => {
                    drop(model);
                    emit(ListEvent::Move(1));
                    return true;
                }
                Key::Up => row = row.saturating_sub(1),
                Key::Down => row = (row + 1).min(len - 1),
                Key::PageUp => row = row.saturating_sub(visible as usize),
                Key::PageDown => row = (row + visible as usize).min(len - 1),
                Key::Home => row = 0,
                Key::End => row = len - 1,
                Key::Enter => {
                    drop(model);
                    emit(ListEvent::Activate(row));
                    return true;
                }
                k if ctrl && k == Key::from_char('a') => {
                    model.sel = (0..len).collect();
                    drop(model);
                    w.redraw();
                    emit(ListEvent::SelectAll);
                    return true;
                }
                _ => return false,
            }
            model.sel.clear();
            model.sel.insert(row);
            model.focus = row;
            model.ensure_visible(row, hh);
            drop(model);
            w.redraw();
            emit(ListEvent::Select(row));
            true
        }
        _ => false,
    }
}

/// Width of the custom list scrollbar.
const SCROLL_W: i32 = 14;

fn scroll_top(pos: i32, body_h: i32, thumb: i32, max: i32) -> i32 {
    let span = (body_h - thumb).max(1) as f64;
    (pos.clamp(0, body_h - thumb) as f64 / span * max as f64).round() as i32
}

/// Scrollbar end-cap size and the usable thumb track between them.
fn scroll_region(body_h: i32) -> (i32, i32) {
    let arrow = SCROLL_W.min((body_h / 3).max(0));
    (arrow, (body_h - 2 * arrow).max(1))
}

/// Scrollbar thumb height and offset within the body area.
fn scroll_thumb(body_h: i32, len: i32, visible: i32, top: i32) -> (i32, i32) {
    let ratio = visible as f64 / len as f64;
    let thumb = ((body_h as f64 * ratio) as i32).clamp(28.min(body_h), body_h);
    let denom = (len - visible).max(1) as f64;
    let off = (body_h - thumb) as f64 * (top as f64 / denom);
    (thumb, off.clamp(0.0, (body_h - thumb) as f64) as i32)
}

/// Column rectangles: name absorbs the leftover space.
fn col_rects(model: &ListModel, width: i32) -> Vec<(Col, i32, i32)> {
    let bar = if model.scrollbar { SCROLL_W } else { 0 };
    let fixed: i32 = model
        .cols
        .iter()
        .filter(|c| !matches!(c, Col::Name | Col::Info))
        .map(|c| c.width(width, 0))
        .sum();
    let remain = (width - bar - fixed).max(160);
    let has_info = model.cols.contains(&Col::Info);
    let info_w = if has_info { (remain * 42 / 100).max(96) } else { 0 };
    let name_w = (remain - info_w).max(140);
    let mut x = 0;
    model
        .cols
        .iter()
        .map(|c| {
            let cw = match c {
                Col::Name => name_w,
                Col::Info => info_w,
                other => other.width(width, 0),
            };
            let out = (*c, x, cw);
            x += cw;
            out
        })
        .collect()
}

fn col_at(model: &ListModel, mx: i32, width: i32) -> (Option<Col>, i32) {
    for (c, x, cw) in col_rects(model, width) {
        if mx >= x && mx < x + cw {
            return (Some(c), x);
        }
    }
    (None, 0)
}

fn draw_list(w: &mut Widget, model: &mut ListModel) {
    let pal = model.pal;
    let (x, y, ww, hh) = (w.x(), w.y(), w.w(), w.h());
    draw_rect_fill(x, y, ww, hh, pal.list_bg);
    let header = if model.show_header { model.header_h } else { 0 };
    let body_h = (hh - header).max(1);
    let visible = model.visible_rows(hh);
    let len = model.tracks.len();

    if model.show_header {
        draw_rect_fill(x, y, ww, header, pal.header);
        set_font(Font::HelveticaBold, 11);
        for (c, cx, cw) in col_rects(model, ww) {
            set_draw_color(pal.header_text);
            draw_text2(c.label(), x + cx + 7, y, cw - 12, header, c.align());
            if c.key() == Some(model.sort) {
                let ax = x + cx + cw - 9;
                let ay = y + header / 2;
                let d = if model.sort_desc { -1 } else { 1 };
                set_draw_color(pal.header_text);
                fltk::draw::draw_polygon(
                    ax,
                    ay - 3 * d,
                    ax + 6,
                    ay - 3 * d,
                    ax + 3,
                    ay + 3 * d,
                );
            }
        }
        set_line_style(LineStyle::Solid, 1);
        set_draw_color(pal.grid);
        fltk::draw::draw_line(x, y + header - 1, x + ww, y + header - 1);
        for (_, cx, _) in col_rects(model, ww) {
            if cx > 0 {
                fltk::draw::draw_line(x + cx, y + 4, x + cx, y + header - 4);
            }
        }
    }

    if model.top > (len as i32 - visible).max(0) {
        model.top = (len as i32 - visible).max(0);
    }

    push_clip(x, y + header, ww, body_h);
    let bar = model.scrollbar && len as i32 > visible;
    let rw = if bar { ww - SCROLL_W } else { ww };
    for i in 0..=visible {
        let row = model.top + i;
        if row < 0 || row as usize >= len {
            break;
        }
        let row = row as usize;
        let ry = y + header + i * model.rowh;
        let selected = model.sel.contains(&row);
        let current = model.cur == Some(row);
        let bg = if current {
            pal.current
        } else if selected {
            pal.list_sel
        } else if model.month_colors {
            crate::theme::rgb(month_tint(model.row_month(row)))
        } else if i % 2 == 1 {
            pal.list_bg_alt
        } else {
            pal.list_bg
        };
        let tx = if selected && !current {
            pal.list_sel_text
        } else if current {
            pal.current_text
        } else {
            pal.list_text
        };
        if selected && model.full_row {
            draw_rect_fill(x, ry, rw, model.rowh, bg);
        } else if current || i % 2 == 1 || model.month_colors {
            draw_rect_fill(x, ry, rw, model.rowh, bg);
        }

        set_font(Font::Helvetica, 12);
        for (c, cx, cw) in col_rects(model, ww) {
            let text = model.text(row, c);
            set_draw_color(tx);
            draw_text2(&text, x + cx + 7, ry, cw - 12, model.rowh, c.align());
        }
        if model.grid {
            set_draw_color(pal.grid);
            fltk::draw::draw_line(x, ry + model.rowh - 1, x + rw, ry + model.rowh - 1);
        }
        if !model.full_row && selected {
            set_draw_color(pal.list_sel);
            fltk::draw::draw_rect(x + 1, ry + 1, rw - 2, model.rowh - 2);
        }
    }
    pop_clip();

    if len == 0 {
        set_draw_color(pal.dim);
        draw_text2(&model.empty_text, x, y + header, ww, body_h, Align::Center);
    }

    if model.scrollbar && len as i32 > visible {
        let sx = x + ww - SCROLL_W;
        let (arrow, track) = scroll_region(body_h);
        draw_box(FrameType::FlatBox, sx, y + header, SCROLL_W, body_h, pal.header);
        draw_box(FrameType::UpBox, sx, y + header, SCROLL_W, arrow, pal.button);
        draw_box(
            FrameType::UpBox,
            sx,
            y + header + body_h - arrow,
            SCROLL_W,
            arrow,
            pal.button,
        );
        let (thumb, off) = scroll_thumb(track, len as i32, visible, model.top);
        let ty = y + header + arrow + off;
        draw_box(FrameType::UpBox, sx, ty, SCROLL_W, thumb, pal.button);
        let cx = sx + SCROLL_W / 2;
        set_draw_color(pal.text);
        fltk::draw::draw_polygon(
            cx,
            y + header + arrow / 2 - 2,
            cx - 3,
            y + header + arrow / 2 + 2,
            cx + 3,
            y + header + arrow / 2 + 2,
        );
        let by = y + header + body_h - arrow / 2;
        fltk::draw::draw_polygon(cx, by + 2, cx - 3, by - 2, cx + 3, by - 2);
    }

    // Pane border drawn last so it stays crisp over the rows.
    set_draw_color(pal.grid);
    set_line_style(LineStyle::Solid, 1);
    fltk::draw::draw_rect(x, y, ww, hh);
}
