#![allow(dead_code)]

use fltk::{
    draw::{
        draw_line, draw_polygon, draw_rect_fill, draw_rectf_with_rgb, set_line_style, LineStyle,
    },
    enums::Color,
};

/// Toolbar and list glyphs, drawn from primitives on a 100x100 grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Menu,
    Search,
    Explore,
    Intro,
    App1,
    App2,
    App3,
    Http,
    Play,
    Stop,
    Pause,
    Prev,
    Next,
    PrevDir,
    NextDir,
    RepeatOff,
    RepeatAll,
    RepeatOne,
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
    Bookmark,
    Goto,
    Find,
    QuitNext,
    BigView,
    Log,
    Picture,
    Mute,
    Info,
    Folder,
    File,
    Cue,
}

fn pt(x: i32, y: i32, w: i32, h: i32, fx: f64, fy: f64) -> (i32, i32) {
    (
        x + (fx * w as f64 / 100.0).round() as i32,
        y + (fy * h as f64 / 100.0).round() as i32,
    )
}

fn line(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    a: (f64, f64),
    b: (f64, f64),
    color: Color,
    thick: i32,
) {
    let (x1, y1) = pt(x, y, w, h, a.0, a.1);
    let (x2, y2) = pt(x, y, w, h, b.0, b.1);
    set_line_style(LineStyle::Solid, thick.max(1));
    fltk::draw::set_draw_color(color);
    draw_line(x1, y1, x2, y2);
    set_line_style(LineStyle::Solid, 1);
}

fn tri(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    color: Color,
) {
    let p1 = pt(x, y, w, h, a.0, a.1);
    let p2 = pt(x, y, w, h, b.0, b.1);
    let p3 = pt(x, y, w, h, c.0, c.1);
    fltk::draw::set_draw_color(color);
    draw_polygon(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
}

fn boxf(x: i32, y: i32, w: i32, h: i32, a: (f64, f64), b: (f64, f64), color: Color) {
    let (x1, y1) = pt(x, y, w, h, a.0, a.1);
    let (x2, y2) = pt(x, y, w, h, b.0, b.1);
    draw_rect_fill(x1, y1, x2 - x1, y2 - y1, color);
}

fn outline(x: i32, y: i32, w: i32, h: i32, a: (f64, f64), b: (f64, f64), color: Color, t: i32) {
    let (x1, y1) = pt(x, y, w, h, a.0, a.1);
    let (x2, y2) = pt(x, y, w, h, b.0, b.1);
    set_line_style(LineStyle::Solid, t.max(1));
    fltk::draw::set_draw_color(color);
    fltk::draw::draw_rect(x1, y1, x2 - x1, y2 - y1);
    set_line_style(LineStyle::Solid, 1);
}

fn circle(cx: f64, cy: f64, r: f64, color: Color, thick: i32) {
    let steps = 32.max((r as i32) * 2);
    let mut prev = (cx + r, cy);
    for i in 1..=steps {
        let a = i as f64 * std::f64::consts::TAU / steps as f64;
        let p = (cx + r * a.cos(), cy + r * a.sin());
        fltk::draw::set_draw_color(color);
        set_line_style(LineStyle::Solid, thick.max(1));
        fltk::draw::draw_line(prev.0 as i32, prev.1 as i32, p.0 as i32, p.1 as i32);
        prev = p;
    }
    set_line_style(LineStyle::Solid, 1);
}

fn arc(
    cx: f64,
    cy: f64,
    r: f64,
    from: f64,
    to: f64,
    color: Color,
    thick: i32,
    steps: i32,
) {
    let mut prev = (cx + r * from.cos(), cy + r * from.sin());
    for i in 1..=steps {
        let a = from + (to - from) * i as f64 / steps as f64;
        let p = (cx + r * a.cos(), cy + r * a.sin());
        fltk::draw::set_draw_color(color);
        set_line_style(LineStyle::Solid, thick.max(1));
        fltk::draw::draw_line(prev.0 as i32, prev.1 as i32, p.0 as i32, p.1 as i32);
        prev = p;
    }
    set_line_style(LineStyle::Solid, 1);
}

fn circle_in(x: i32, y: i32, w: i32, h: i32, cx: f64, cy: f64, r: f64, color: Color, t: i32) {
    let c = pt(x, y, w, h, cx, cy);
    let rr = (r * w as f64 / 100.0).max(2.0);
    circle(c.0 as f64, c.1 as f64, rr, color, t);
}

/// Draw `icon` inside the box. `color` is the foreground.
pub fn draw(icon: Icon, x: i32, y: i32, w: i32, h: i32, color: Color) {
    let t = ((w as f64 / 10.0).round() as i32).max(1);
    fltk::draw::set_draw_color(color);
    match icon {
        Icon::Menu => {
            for yy in [30.0, 50.0, 70.0] {
                line(x, y, w, h, (20.0, yy), (80.0, yy), color, t);
            }
        }
        Icon::Search | Icon::Find => {
            circle_in(x, y, w, h, 42.0, 42.0, 24.0, color, t);
            line(x, y, w, h, (60.0, 60.0), (82.0, 82.0), color, t + 1);
        }
        Icon::Explore | Icon::Folder => {
            line(x, y, w, h, (14.0, 36.0), (14.0, 78.0), color, t);
            line(x, y, w, h, (14.0, 78.0), (86.0, 78.0), color, t);
            line(x, y, w, h, (86.0, 78.0), (86.0, 24.0), color, t);
            line(x, y, w, h, (86.0, 24.0), (44.0, 24.0), color, t);
            line(x, y, w, h, (44.0, 24.0), (36.0, 36.0), color, t);
            line(x, y, w, h, (36.0, 36.0), (14.0, 36.0), color, t);
        }
        Icon::Intro => {
            // Headphones, the audible preview symbol.
            arc(
                x as f64 + w as f64 * 0.5,
                y as f64 + h as f64 * 0.55,
                w as f64 * 0.36,
                std::f64::consts::PI,
                std::f64::consts::TAU,
                color,
                t,
                20,
            );
            boxf(x, y, w, h, (16.0, 50.0), (28.0, 78.0), color);
            boxf(x, y, w, h, (72.0, 50.0), (84.0, 78.0), color);
        }
        Icon::App1 | Icon::App2 | Icon::App3 => {
            let count = match icon {
                Icon::App1 => 1,
                Icon::App2 => 2,
                _ => 3,
            };
            for i in 0..count {
                let d = 24.0 - i as f64 * 8.0;
                outline(x, y, w, h, (d, d), (100.0 - d, 100.0 - d), color, 1);
                line(x, y, w, h, (d, d + 14.0), (100.0 - d, d + 14.0), color, 1);
            }
        }
        Icon::Http => {
            circle_in(x, y, w, h, 50.0, 50.0, 30.0, color, t);
            line(x, y, w, h, (20.0, 50.0), (80.0, 50.0), color, 1);
            line(x, y, w, h, (50.0, 20.0), (50.0, 80.0), color, 1);
        }
        Icon::Play => {
            tri(x, y, w, h, (28.0, 18.0), (28.0, 82.0), (80.0, 50.0), color);
        }
        Icon::Pause => {
            boxf(x, y, w, h, (27.0, 20.0), (43.0, 80.0), color);
            boxf(x, y, w, h, (57.0, 20.0), (73.0, 80.0), color);
        }
        Icon::Stop | Icon::Mute => {
            boxf(x, y, w, h, (28.0, 28.0), (72.0, 72.0), color);
            if icon == Icon::Mute {
                line(x, y, w, h, (18.0, 18.0), (82.0, 82.0), Color::Red, t);
            }
        }
        Icon::Prev | Icon::Next => {
            let flip = icon == Icon::Prev;
            let bar_x = if flip { 24.0 } else { 76.0 };
            line(x, y, w, h, (bar_x, 20.0), (bar_x, 80.0), color, t + 1);
            if flip {
                tri(x, y, w, h, (78.0, 20.0), (78.0, 80.0), (40.0, 50.0), color);
            } else {
                tri(x, y, w, h, (22.0, 20.0), (22.0, 80.0), (60.0, 50.0), color);
            }
        }
        Icon::PrevDir | Icon::NextDir => {
            let flip = icon == Icon::PrevDir;
            for i in 0..2 {
                let o = i as f64 * 24.0;
                let a = if flip { (66.0 - o, 24.0) } else { (34.0 + o, 24.0) };
                let b = if flip { (66.0 - o, 76.0) } else { (34.0 + o, 76.0) };
                let c = if flip { (34.0 - o, 50.0) } else { (66.0 + o, 50.0) };
                tri(x, y, w, h, a, b, c, color);
            }
        }
        Icon::RepeatOff | Icon::RepeatAll | Icon::RepeatOne => {
            let on = icon != Icon::RepeatOff;
            let col = if on { color } else { Color::from_rgb(135, 135, 135) };
            // Open loop of two arrows.
            line(x, y, w, h, (26.0, 32.0), (74.0, 32.0), col, t);
            line(x, y, w, h, (74.0, 32.0), (74.0, 46.0), col, t);
            line(x, y, w, h, (74.0, 68.0), (26.0, 68.0), col, t);
            line(x, y, w, h, (26.0, 68.0), (26.0, 54.0), col, t);
            tri(x, y, w, h, (70.0, 22.0), (86.0, 32.0), (70.0, 42.0), col);
            tri(x, y, w, h, (30.0, 58.0), (14.0, 68.0), (30.0, 78.0), col);
            if icon == Icon::RepeatOne {
                line(x, y, w, h, (50.0, 44.0), (50.0, 58.0), col, t);
            }
        }
        Icon::Shuffle => {
            line(x, y, w, h, (16.0, 26.0), (44.0, 50.0), color, t);
            line(x, y, w, h, (44.0, 50.0), (80.0, 74.0), color, t);
            line(x, y, w, h, (16.0, 74.0), (44.0, 50.0), color, t);
            line(x, y, w, h, (44.0, 50.0), (80.0, 26.0), color, t);
            tri(x, y, w, h, (72.0, 18.0), (88.0, 26.0), (72.0, 34.0), color);
            tri(x, y, w, h, (72.0, 66.0), (88.0, 74.0), (72.0, 82.0), color);
        }
        Icon::Settings => {
            circle_in(x, y, w, h, 50.0, 50.0, 14.0, color, t);
            for k in 0..8 {
                let a = k as f64 * std::f64::consts::TAU / 8.0;
                let (s, c) = (a.sin(), a.cos());
                line(
                    x,
                    y,
                    w,
                    h,
                    (50.0 + c * 24.0, 50.0 + s * 24.0),
                    (50.0 + c * 34.0, 50.0 + s * 34.0),
                    color,
                    t,
                );
            }
        }
        Icon::Compact => {
            line(x, y, w, h, (18.0, 34.0), (58.0, 34.0), color, t);
            line(x, y, w, h, (18.0, 66.0), (58.0, 66.0), color, t);
            line(x, y, w, h, (76.0, 24.0), (76.0, 76.0), color, 1);
            tri(x, y, w, h, (68.0, 32.0), (84.0, 32.0), (76.0, 20.0), color);
            tri(x, y, w, h, (68.0, 68.0), (84.0, 68.0), (76.0, 80.0), color);
        }
        Icon::Tree => {
            line(x, y, w, h, (26.0, 22.0), (26.0, 74.0), color, 1);
            line(x, y, w, h, (26.0, 22.0), (48.0, 22.0), color, 1);
            line(x, y, w, h, (26.0, 48.0), (48.0, 48.0), color, 1);
            line(x, y, w, h, (26.0, 74.0), (48.0, 74.0), color, 1);
            boxf(x, y, w, h, (16.0, 14.0), (26.0, 30.0), color);
            outline(x, y, w, h, (48.0, 40.0), (62.0, 56.0), color, 1);
            outline(x, y, w, h, (48.0, 66.0), (62.0, 82.0), color, 1);
        }
        Icon::TrackNext => {
            line(x, y, w, h, (50.0, 18.0), (50.0, 62.0), color, t);
            tri(x, y, w, h, (34.0, 54.0), (66.0, 54.0), (50.0, 80.0), color);
            line(x, y, w, h, (26.0, 86.0), (74.0, 86.0), color, t);
        }
        Icon::Skip => {
            // Note with a red slash: skipped from the list, file untouched.
            circle_in(x, y, w, h, 38.0, 66.0, 11.0, color, t);
            line(x, y, w, h, (48.0, 66.0), (48.0, 24.0), color, t);
            line(x, y, w, h, (48.0, 24.0), (64.0, 34.0), color, t);
            line(x, y, w, h, (16.0, 20.0), (84.0, 80.0), Color::from_rgb(190, 55, 55), t + 1);
        }
        Icon::Playlist => {
            for yy in [24.0, 42.0, 60.0] {
                line(x, y, w, h, (14.0, yy), (54.0, yy), color, t);
            }
            boxf(x, y, w, h, (60.0, 66.0), (74.0, 78.0), color);
            line(x, y, w, h, (74.0, 24.0), (74.0, 70.0), color, t);
            line(x, y, w, h, (74.0, 24.0), (88.0, 32.0), color, t);
        }
        Icon::Reload => {
            let cx = pt(x, y, w, h, 50.0, 50.0);
            let r = (30.0 * w as f64 / 100.0).max(3.0);
            arc(
                cx.0 as f64,
                cx.1 as f64,
                r,
                0.55,
                std::f64::consts::TAU - 0.95,
                color,
                t,
                22,
            );
            tri(x, y, w, h, (62.0, 16.0), (84.0, 24.0), (64.0, 38.0), color);
        }
        Icon::Home => {
            line(x, y, w, h, (12.0, 50.0), (50.0, 16.0), color, t);
            line(x, y, w, h, (50.0, 16.0), (88.0, 50.0), color, t);
            line(x, y, w, h, (24.0, 46.0), (24.0, 84.0), color, t);
            line(x, y, w, h, (76.0, 46.0), (76.0, 84.0), color, t);
            line(x, y, w, h, (24.0, 84.0), (76.0, 84.0), color, t);
            outline(x, y, w, h, (42.0, 62.0), (58.0, 84.0), color, 1);
        }
        Icon::Scan => {
            circle_in(x, y, w, h, 42.0, 42.0, 22.0, color, t);
            line(x, y, w, h, (58.0, 58.0), (82.0, 82.0), color, t + 1);
            for yy in [(34.0, 52.0), (42.0, 32.0), (50.0, 46.0)] {
                line(x, y, w, h, (yy.0, 50.0), (yy.0, yy.1), color, 1);
            }
        }
        Icon::Bookmark => {
            tri(x, y, w, h, (30.0, 16.0), (70.0, 16.0), (70.0, 86.0), color);
            tri(x, y, w, h, (30.0, 16.0), (30.0, 86.0), (50.0, 64.0), color);
            tri(x, y, w, h, (50.0, 64.0), (70.0, 86.0), (70.0, 16.0), color);
        }
        Icon::Goto => {
            line(x, y, w, h, (18.0, 24.0), (18.0, 76.0), color, t);
            line(x, y, w, h, (18.0, 24.0), (32.0, 24.0), color, t);
            line(x, y, w, h, (18.0, 76.0), (32.0, 76.0), color, t);
            line(x, y, w, h, (38.0, 50.0), (76.0, 50.0), color, t);
            tri(x, y, w, h, (68.0, 36.0), (88.0, 50.0), (68.0, 64.0), color);
        }
        Icon::QuitNext => {
            line(x, y, w, h, (22.0, 20.0), (22.0, 80.0), color, t);
            line(x, y, w, h, (22.0, 20.0), (40.0, 20.0), color, t);
            line(x, y, w, h, (22.0, 80.0), (40.0, 80.0), color, t);
            line(x, y, w, h, (46.0, 50.0), (80.0, 50.0), color, t);
            tri(x, y, w, h, (70.0, 36.0), (90.0, 50.0), (70.0, 64.0), color);
        }
        Icon::BigView => {
            line(x, y, w, h, (50.0, 14.0), (26.0, 70.0), color, t + 1);
            line(x, y, w, h, (50.0, 14.0), (74.0, 70.0), color, t + 1);
            line(x, y, w, h, (36.0, 50.0), (64.0, 50.0), color, t);
            line(x, y, w, h, (26.0, 84.0), (74.0, 84.0), color, t);
        }
        Icon::Log => {
            outline(x, y, w, h, (24.0, 14.0), (76.0, 86.0), color, 1);
            for yy in [32.0, 48.0, 64.0] {
                line(x, y, w, h, (34.0, yy), (66.0, yy), color, 1);
            }
        }
        Icon::Picture => {
            outline(x, y, w, h, (14.0, 22.0), (86.0, 78.0), color, 1);
            tri(x, y, w, h, (28.0, 70.0), (44.0, 46.0), (58.0, 70.0), color);
            tri(x, y, w, h, (54.0, 70.0), (66.0, 54.0), (80.0, 70.0), color);
            circle_in(x, y, w, h, 70.0, 36.0, 5.0, color, 1);
        }
        Icon::Info => {
            circle_in(x, y, w, h, 50.0, 50.0, 30.0, color, t);
            line(x, y, w, h, (50.0, 44.0), (50.0, 72.0), color, t);
            boxf(x, y, w, h, (46.0, 30.0), (54.0, 38.0), color);
        }
        Icon::File | Icon::Cue => {
            line(x, y, w, h, (28.0, 14.0), (62.0, 14.0), color, 1);
            line(x, y, w, h, (62.0, 14.0), (76.0, 30.0), color, 1);
            line(x, y, w, h, (76.0, 30.0), (76.0, 86.0), color, 1);
            line(x, y, w, h, (76.0, 86.0), (28.0, 86.0), color, 1);
            line(x, y, w, h, (28.0, 86.0), (28.0, 14.0), color, 1);
            if icon == Icon::Cue {
                line(x, y, w, h, (38.0, 62.0), (66.0, 62.0), color, t);
                line(x, y, w, h, (38.0, 74.0), (56.0, 74.0), color, 1);
            }
        }
    }
}

/// Small filled dot, used by list decoration.
pub fn dot(x: i32, y: i32, d: i32, color: Color) {
    let r = (d / 2).max(1);
    for dy in -r..=r {
        let span = ((r * r - dy * dy) as f64).sqrt() as i32;
        fltk::draw::set_draw_color(color);
        draw_rect_fill(x - span, y + dy, span * 2 + 1, 1, color);
    }
    let _ = draw_rectf_with_rgb;
}
