//! Lucide icon set, rendered from embedded SVG by FLTK at display resolution.
//! Lucide is ISC licensed; see assets/icons/LICENSE.

use std::collections::HashMap;

use fltk::{
    image::SvgImage,
    prelude::*,
};

use crate::icons::Icon;

fn svg(icon: Icon) -> &'static str {
    match icon {
        Icon::Menu => include_str!("../assets/icons-src/menu.svg"),
        Icon::Search | Icon::Find => include_str!("../assets/icons-src/search.svg"),
        Icon::Explore => include_str!("../assets/icons-src/folder-open.svg"),
        Icon::Folder => include_str!("../assets/icons-src/folder.svg"),
        Icon::Intro => include_str!("../assets/icons-src/headphones.svg"),
        Icon::App1 | Icon::App2 | Icon::App3 => include_str!("../assets/icons-src/rows-3.svg"),
        Icon::Http => include_str!("../assets/icons-src/globe.svg"),
        Icon::Play => include_str!("../assets/icons-src/play.svg"),
        Icon::Stop => include_str!("../assets/icons-src/square.svg"),
        Icon::Pause => include_str!("../assets/icons-src/pause.svg"),
        Icon::Mute => include_str!("../assets/icons-src/volume-x.svg"),
        Icon::Prev => include_str!("../assets/icons-src/skip-back.svg"),
        Icon::Next => include_str!("../assets/icons-src/skip-forward.svg"),
        Icon::PrevDir => include_str!("../assets/icons-src/chevrons-left.svg"),
        Icon::NextDir => include_str!("../assets/icons-src/chevrons-right.svg"),
        Icon::RepeatOff | Icon::RepeatAll => include_str!("../assets/icons-src/repeat-2.svg"),
        Icon::RepeatOne => include_str!("../assets/icons-src/repeat-1.svg"),
        Icon::Shuffle => include_str!("../assets/icons-src/shuffle.svg"),
        Icon::Settings => include_str!("../assets/icons-src/settings.svg"),
        Icon::Compact => include_str!("../assets/icons-src/rows-3.svg"),
        Icon::Tree => include_str!("../assets/icons-src/list-tree.svg"),
        Icon::TrackNext => include_str!("../assets/icons-src/arrow-down-to-line.svg"),
        Icon::Skip => include_str!("../assets/icons-src/list-x.svg"),
        Icon::Playlist => include_str!("../assets/icons-src/list-music.svg"),
        Icon::Reload => include_str!("../assets/icons-src/rotate-cw.svg"),
        Icon::Home => include_str!("../assets/icons-src/house.svg"),
        Icon::Scan => include_str!("../assets/icons-src/scan-search.svg"),
        Icon::Bookmark => include_str!("../assets/icons-src/bookmark.svg"),
        Icon::Goto => include_str!("../assets/icons-src/arrow-right-to-line.svg"),
        Icon::QuitNext => include_str!("../assets/icons-src/power.svg"),
        Icon::BigView => include_str!("../assets/icons-src/type.svg"),
        Icon::Log => include_str!("../assets/icons-src/scroll-text.svg"),
        Icon::Picture => include_str!("../assets/icons-src/image.svg"),
        Icon::Info => include_str!("../assets/icons-src/info.svg"),
        Icon::File => include_str!("../assets/icons-src/file.svg"),
        Icon::Cue => include_str!("../assets/icons-src/file-text.svg"),
    }
}

fn load(icon: Icon, dark_theme: bool) -> Option<SvgImage> {
    let ink = if dark_theme { "#E6E6E6" } else { "#333333" };
    let body = svg(icon).replace("currentColor", ink);
    SvgImage::from_data(&body).ok()
}

thread_local! {
    static CACHE: std::cell::RefCell<HashMap<(Icon, bool, i32, i32), Option<SvgImage>>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Draw the icon at the given logical size. FLTK rasterises the SVG at the
/// display's pixel density, so edges stay sharp on Retina screens.
pub fn draw(icon: Icon, x: i32, y: i32, w: i32, h: i32, dark_theme: bool) -> bool {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let img = c.entry((icon, dark_theme, w, h)).or_insert_with(|| {
            load(icon, dark_theme).map(|mut img| {
                // Size the SVG before drawing. Otherwise FLTK keeps the 24 px
                // viewBox and the glyph is clipped by the widget.
                img.scale(w, h, false, true);
                img
            })
        });
        match img {
            Some(img) => {
                img.draw(x, y, w, h);
                true
            }
            None => false,
        }
    })
}
