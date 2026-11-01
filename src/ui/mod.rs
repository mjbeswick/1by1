use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
};

use fltk::{
    draw::{draw_rect_fill, draw_text2, set_draw_color, set_font},
    enums::{Align, Font, FrameType},
    frame::Frame,
    image::SharedImage,
    menu::SysMenuBar,
    prelude::*,
    tree::Tree,
    widget::Widget,
    window::Window,
};

use crate::{
    settings::{CoverMode, PlaylistPos, Settings},
    theme::Palette,
};

use self::{
    track_list::{ListModel, TList},
    widgets::{BigModel, BigView, FlatSlider, SliderModel, Toolbar, ToolbarModel},
};

pub mod dialogs;
pub mod menus;
pub mod track_list;
pub mod widgets;

pub const MARGIN: i32 = 8;
pub const TOOLBAR_H: i32 = 30;
pub const SEEK_H: i32 = 16;
pub const STATUS_H: i32 = 22;
pub const GAP: i32 = 6;
pub const TREE_W: i32 = 240;
pub const COVER_H: i32 = 160;

/// Cover art pane below the folder tree.
#[derive(Clone)]
pub struct CoverView {
    pub wid: Widget,
    pub image: Rc<RefCell<Option<SharedImage>>>,
    pub pal: Rc<RefCell<Palette>>,
    pub text: Rc<RefCell<String>>,
    pub mode: Rc<Cell<CoverMode>>,
}

impl CoverView {
    pub fn new(x: i32, y: i32, w: i32, h: i32, pal: Palette) -> Self {
        let mut wid = Widget::new(x, y, w, h, "");
        wid.set_frame(FrameType::FlatBox);
        let image: Rc<RefCell<Option<SharedImage>>> = Rc::new(RefCell::new(None));
        let pal = Rc::new(RefCell::new(pal));
        let text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
        let mode = Rc::new(Cell::new(CoverMode::Artwork));
        let im = image.clone();
        let p = pal.clone();
        let t = text.clone();
        let md = mode.clone();
        wid.draw(move |w| {
            let (x, y, ww, hh) = (w.x(), w.y(), w.w(), w.h());
            let pal = *p.borrow();
            draw_rect_fill(x, y, ww, hh, pal.list_bg);
            let mut drew = false;
            if let Some(img) = im.borrow_mut().as_mut() {
                if md.get() == CoverMode::Milkdrop {
                    if img.width() == ww.max(1) && img.height() == hh.max(1) {
                        img.draw(x, y, ww.max(1), hh.max(1));
                    } else {
                        let mut copy = img.copy();
                        copy.scale(ww.max(1), hh.max(1), false, true);
                        copy.draw(x, y, ww.max(1), hh.max(1));
                    }
                } else {
                    let pad = 6;
                    let ratio = ((ww - pad * 2) as f64 / img.width() as f64)
                        .min((hh - pad * 2) as f64 / img.height() as f64);
                    let dw = (img.width() as f64 * ratio) as i32;
                    let dh = (img.height() as f64 * ratio) as i32;
                    let dx = x + (ww - dw) / 2;
                    let dy = y + (hh - dh) / 2;
                    let mut copy = img.copy();
                    copy.scale(dw.max(1), dh.max(1), false, true);
                    copy.draw(dx, dy, dw.max(1), dh.max(1));
                }
                drew = true;
            }
            if !drew {
                set_font(Font::Helvetica, 11);
                set_draw_color(pal.dim);
                draw_text2(
                    &t.borrow(),
                    x + 8,
                    y + hh / 2 - 16,
                    ww - 16,
                    32,
                    Align::Center | Align::Inside,
                );
            }
            set_draw_color(pal.grid);
            fltk::draw::draw_rect(x, y, ww, hh);
        });
        Self {
            wid,
            image,
            pal,
            text,
            mode,
        }
    }

    pub fn set_mode(&self, mode: CoverMode) {
        self.mode.set(mode);
        self.wid.clone().redraw();
    }

    pub fn set_image(&self, img: Option<SharedImage>) {
        *self.image.borrow_mut() = img;
        self.wid.clone().redraw();
    }

    pub fn set_text(&self, s: &str) {
        *self.text.borrow_mut() = s.to_string();
        self.wid.clone().redraw();
    }
}

/// A panel boundary that can be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitKind {
    Tree,
    Cover,
    Playlist,
}

/// Thin strip between panels. Cursor and line direction follow `north_south`.
#[derive(Clone)]
pub struct Splitter {
    pub wid: Widget,
    north_south: Rc<Cell<bool>>,
}

impl Splitter {
    fn new(x: i32, y: i32, w: i32, h: i32, north_south: bool, pal: Palette) -> Self {
        let mut wid = Widget::new(x, y, w, h, "");
        wid.set_frame(FrameType::NoBox);
        let ns = Rc::new(Cell::new(north_south));
        let n = ns.clone();
        wid.draw(move |w| {
            let (x, y, ww, hh) = (w.x(), w.y(), w.w(), w.h());
            draw_rect_fill(x, y, ww, hh, pal.window);
            set_draw_color(pal.grid);
            if n.get() {
                draw_rect_fill(x, y + hh / 2, ww, 1, pal.grid);
            } else {
                draw_rect_fill(x + ww / 2, y, 1, hh, pal.grid);
            }
        });
        Self {
            wid,
            north_south: ns,
        }
    }

    pub fn set_north_south(&mut self, north_south: bool) {
        self.north_south.set(north_south);
        self.wid.redraw();
    }
}

pub struct Ui {
    pub win: Window,
    pub bar: Toolbar,
    pub seek: FlatSlider,
    pub vol: FlatSlider,
    pub tree: Tree,
    pub tree_back: Frame,
    pub files: TList,
    pub playlist: TList,
    pub big: BigView,
    pub status: Frame,
    pub flags: Frame,
    pub status_line: Frame,
    pub cover: CoverView,
    pub cover_visible: bool,
    pub tree_split: Splitter,
    pub cover_split: Splitter,
    pub playlist_split: Splitter,
    pub menu_bar: Option<SysMenuBar>,
    /// Tree pathname to folder on disk.
    pub paths: HashMap<String, PathBuf>,
}

impl Ui {
    pub fn set_cover_visible(&mut self, s: &Settings, visible: bool) {
        if self.cover_visible != visible {
            self.cover_visible = visible;
            self.relayout(s);
        }
    }

    pub fn relayout(&mut self, s: &Settings) {
        let (ww, hh) = (self.win.w(), self.win.h());
        if s.compact {
            self.bar.wid.set_pos(MARGIN, 3);
            self.bar.wid.set_size(ww - MARGIN * 2, 30);
            self.seek.wid.hide();
            self.vol.wid.hide();
            self.tree.hide();
            self.tree_back.hide();
            self.files.wid.hide();
            self.playlist.wid.hide();
            self.big.wid.hide();
            self.cover.wid.hide();
            self.tree_split.wid.hide();
            self.cover_split.wid.hide();
            self.playlist_split.wid.hide();
            self.status.hide();
            self.flags.hide();
            self.status_line.hide();
            return;
        }
        self.bar.wid.set_pos(MARGIN, 4);
        self.bar.wid.set_size(ww - MARGIN * 2, TOOLBAR_H);

        let seek_h = if s.larger_sliders { 20 } else { SEEK_H };
        let seek_y = 4 + TOOLBAR_H + 3;
        let vol_w = 112;
        self.seek.wid.show();
        self.vol.wid.show();
        self.seek
            .wid
            .set_pos(MARGIN + 2, seek_y);
        self.seek
            .wid
            .set_size((ww - MARGIN * 2 - vol_w - 12).max(60), seek_h);
        self.vol.wid.set_pos(ww - MARGIN - vol_w, seek_y);
        self.vol.wid.set_size(vol_w, seek_h);

        let status_h = STATUS_H;
        let body_y = seek_y + seek_h + GAP;
        let body_h = (hh - body_y - status_h - MARGIN).max(80);

        self.status_line.show();
        self.status_line.set_pos(0, hh - status_h - 1);
        self.status_line.set_size(ww, 1);

        let tree_w = if s.show_tree {
            s.tree_width.clamp(140, (ww - 200).max(140))
        } else {
            0
        };
        let cover_h = if s.show_tree
            && s.cover_art
            && self.cover_visible
            && s.cover_mode != CoverMode::Off
        {
            s.cover_height.clamp(60, (body_h - 120).max(60))
        } else {
            0
        };
        let gap = if s.show_tree { GAP } else { 0 };
        let files_x = MARGIN + tree_w + gap;
        let files_w = (ww - MARGIN - files_x).max(160);
        let mut files = (files_x, body_y, files_w, body_h);
        let mut list = (0, 0, 0, 0);
        if s.playlist_open {
            match s.playlist_pos {
                PlaylistPos::Top => {
                    let ph = s.playlist_height.clamp(80, (body_h - 120).max(80));
                    list = (files_x, body_y, files_w, ph);
                    files = (files_x, body_y + ph + GAP, files_w, body_h - ph - GAP);
                }
                PlaylistPos::Bottom => {
                    let ph = s.playlist_height.clamp(80, (body_h - 120).max(80));
                    files = (files_x, body_y, files_w, body_h - ph - GAP);
                    list = (files_x, body_y + body_h - ph, files_w, ph);
                }
                PlaylistPos::Right => {
                    let pw = s
                        .playlist_width
                        .clamp(180, (files_w - GAP - 120).max(180));
                    files = (files_x, body_y, files_w - pw - GAP, body_h);
                    list = (ww - MARGIN - pw, body_y, pw, body_h);
                }
            }
        }

        let big_h = if s.big_view {
            (files.3 / 3).clamp(90, 170)
        } else {
            0
        };
        if big_h > 0 {
            self.big.wid.show();
            self.big.wid.set_pos(files.0, files.1 + files.3 - big_h);
            self.big.wid.set_size(files.2, big_h);
            files.3 -= big_h + GAP;
        } else {
            self.big.wid.hide();
        }

        if s.show_tree {
            self.tree_back.show();
            self.tree_back.set_pos(MARGIN, body_y);
            self.tree_back
                .set_size(tree_w, body_h - cover_h - if cover_h > 0 { GAP } else { 0 });
            self.tree.show();
            self.tree.set_pos(MARGIN + 1, body_y + 1);
            self.tree.set_size(
                tree_w - 2,
                body_h - cover_h - if cover_h > 0 { GAP } else { 0 } - 2,
            );
            if cover_h > 0 {
                self.cover.wid.show();
                self.cover.wid.set_pos(MARGIN, body_y + body_h - cover_h);
                self.cover.wid.set_size(tree_w, cover_h);
            } else {
                self.cover.wid.hide();
            }
        } else {
            self.tree.hide();
            self.tree_back.hide();
            self.cover.wid.hide();
        }

        self.files.wid.show();
        self.files
            .resize(files.0, files.1, files.2.max(120), files.3.max(60));
        if s.playlist_open {
            self.playlist.wid.show();
            self.playlist
                .resize(list.0, list.1, list.2.max(120), list.3.max(60));
        } else {
            self.playlist.wid.hide();
        }

        if s.show_tree {
            self.tree_split.wid.show();
            self.tree_split
                .wid
                .set_pos(MARGIN + tree_w, body_y);
            self.tree_split.wid.set_size(GAP, body_h);
            if cover_h > 0 {
                self.cover_split.wid.show();
                self.cover_split
                    .wid
                    .set_pos(MARGIN, body_y + body_h - cover_h - GAP);
                self.cover_split.wid.set_size(tree_w, GAP);
            } else {
                self.cover_split.wid.hide();
            }
        } else {
            self.tree_split.wid.hide();
            self.cover_split.wid.hide();
        }

        if s.playlist_open {
            self.playlist_split.wid.show();
            let right = s.playlist_pos == PlaylistPos::Right;
            self.playlist_split.set_north_south(!right);
            if right {
                self.playlist_split
                    .wid
                    .set_pos(list.0 - GAP, body_y);
                self.playlist_split.wid.set_size(GAP, body_h);
            } else if s.playlist_pos == PlaylistPos::Top {
                self.playlist_split
                    .wid
                    .set_pos(list.0, list.1 + list.3);
                self.playlist_split.wid.set_size(list.2, GAP);
            } else {
                self.playlist_split
                    .wid
                    .set_pos(list.0, list.1 - GAP);
                self.playlist_split.wid.set_size(list.2, GAP);
            }
        } else {
            self.playlist_split.wid.hide();
        }

        self.status.show();
        self.flags.show();
        self.status.set_pos(MARGIN + 4, hh - status_h + 2);
        self.status.set_size(ww - MARGIN * 2 - 330, status_h - 4);
        self.flags.set_pos(ww - MARGIN - 320, hh - status_h + 2);
        self.flags.set_size(320, status_h - 4);
        self.win.redraw();
    }
}

/// Build all widgets. Callbacks are installed by the app afterwards.
pub fn build(s: &Settings, pal: Palette) -> Ui {
    let (ww, hh) = s.win_size;
    let mut win = Window::default()
        .with_size(ww.max(720), hh.max(420))
        .with_label(crate::APP_NAME);
    win.set_color(pal.window);

    let bar_model = Rc::new(RefCell::new(ToolbarModel::new(pal)));
    let bar = Toolbar::new(MARGIN, 4, ww - MARGIN * 2, TOOLBAR_H, bar_model);

    let seek = FlatSlider::new(
        MARGIN,
        37,
        ww - MARGIN * 2 - 124,
        SEEK_H,
        Rc::new(RefCell::new(SliderModel {
            kind: widgets::SliderKind::Position,
            value: 0.0,
            pal,
            dragging: false,
            enabled: false,
        })),
    );
    let vol = FlatSlider::new(
        ww - MARGIN - 112,
        37,
        112,
        SEEK_H,
        Rc::new(RefCell::new(SliderModel {
            kind: widgets::SliderKind::Volume,
            value: s.volume as f64,
            pal,
            dragging: false,
            enabled: true,
        })),
    );

    let mut tree_back = Frame::new(MARGIN, 72, TREE_W, 400, "");
    tree_back.set_frame(FrameType::FlatBox);
    tree_back.set_color(pal.grid);
    let mut tree = Tree::new(MARGIN + 1, 73, TREE_W - 2, 398, None);
    tree.set_frame(FrameType::FlatBox);
    let files = TList::new(
        MARGIN + TREE_W + GAP,
        72,
        ww - TREE_W - GAP - MARGIN * 2,
        400,
        Rc::new(RefCell::new(ListModel::new(pal))),
    );
    let mut pl_model = ListModel::new(pal);
    pl_model.cols = vec![
        track_list::Col::Name,
        track_list::Col::Length,
        track_list::Col::Type,
    ];
    pl_model.empty_text = "Playlist is empty. Enqueue tracks with Ctrl+Return.".into();
    let mut playlist = TList::new(
        MARGIN + TREE_W + GAP,
        320,
        ww - TREE_W - GAP - MARGIN * 2,
        120,
        Rc::new(RefCell::new(pl_model)),
    );
    playlist.wid.hide();

    let mut big = BigView::new(
        MARGIN + TREE_W + GAP,
        420,
        ww - TREE_W - GAP - MARGIN * 2,
        140,
        Rc::new(RefCell::new(BigModel::new(pal))),
    );
    big.wid.hide();

    let mut cover = CoverView::new(MARGIN, 440, TREE_W, COVER_H, pal);
    cover.wid.hide();
    let mut tree_split = Splitter::new(MARGIN + TREE_W, 72, GAP, 400, false, pal);
    let mut cover_split = Splitter::new(MARGIN, 432, TREE_W, GAP, true, pal);
    let mut playlist_split = Splitter::new(MARGIN + TREE_W + GAP, 320, 400, GAP, true, pal);
    tree_split.wid.hide();
    cover_split.wid.hide();
    playlist_split.wid.hide();

    let mut status_line = Frame::new(0, hh - STATUS_H - 1, ww, 1, "");
    status_line.set_frame(FrameType::FlatBox);
    status_line.set_color(pal.grid);

    let mut status = Frame::new(MARGIN + 4, hh - STATUS_H + 2, ww - 350, 18, "Ready");
    status.set_align(Align::Inside | Align::Left);
    status.set_frame(FrameType::FlatBox);
    status.set_color(pal.window);
    status.set_label_color(pal.text);
    let mut flags = Frame::new(ww - MARGIN - 320, hh - STATUS_H + 2, 320, 18, "");
    flags.set_align(Align::Inside | Align::Right);
    flags.set_frame(FrameType::FlatBox);
    flags.set_color(pal.window);
    flags.set_label_color(pal.dim);

    win.end();
    win.make_resizable(true);
    theme_tree(&mut tree, pal, s);
    tree.set_tooltip("Folders: click to load, F5 to rescan, right click on a list for file menus.");
    flags.set_tooltip("F1: main menu. Right click lists and the button bar for context menus.");

    Ui {
        win,
        bar,
        seek,
        vol,
        tree,
        tree_back,
        files,
        playlist,
        big,
        status,
        flags,
        status_line,
        cover,
        cover_visible: true,
        tree_split,
        cover_split,
        playlist_split,
        menu_bar: None,
        paths: HashMap::new(),
    }
}

pub fn theme_tree(tree: &mut Tree, pal: Palette, s: &Settings) {
    tree.set_show_root(false);
    tree.set_frame(FrameType::FlatBox);
    tree.set_color(pal.list_bg);
    tree.set_selection_color(pal.list_sel);
    tree.set_item_label_fgcolor(pal.list_text);
    tree.set_item_label_bgcolor(pal.list_bg);
    tree.set_connector_color(if s.tree_lines { pal.grid } else { pal.list_bg });
    tree.set_item_label_size(12);
    tree.set_margin_top(if s.tree_spacing { 8 } else { 4 });
    tree.set_margin_left(8);
    tree.set_margin_bottom(if s.tree_spacing { 6 } else { 4 });
    tree.set_line_spacing(if s.tree_spacing { 5 } else { 1 });
    tree.set_label_margin_left(if s.tree_spacing { 5 } else { 4 });
    tree.set_connector_width(14);
    tree.set_open_child_margin_bottom(1);
    tree.set_select_frame(FrameType::FlatBox);
    tree.set_scrollbar_size(14);
}
