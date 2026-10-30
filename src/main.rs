use fltk::{app, browser::MultiBrowser, button::Button, frame::Frame, group::{Flex, Tile}, prelude::*, tree::Tree, window::Window};

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut win = Window::default().with_size(900, 600).with_label("1by1-rs");

    let mut col = Flex::default_fill().column();
    col.set_margin(4);

    // Toolbar: transport and mode buttons, unwired.
    let bar = Flex::default().row();
    for label in ["|<", "Play", "Stop", ">|", "Repeat", "Shuffle"] {
        Button::default().with_label(label);
    }
    bar.end();
    col.fixed(&bar, 32);

    // Body: folder tree left, track list right, resizable split.
    let tile = Tile::default_fill();
    let mut tree = Tree::default_fill().with_size(300, 500);
    tree.set_root_label("Folders");
    let _ = tree.add("Music");
    let mut list = MultiBrowser::default_fill();
    list.add("(no folder loaded)");
    tile.end();

    let status = Frame::default().with_label("Ready");
    col.fixed(&status, 22);

    col.end();
    win.end();
    win.resizable(&col);
    win.show();
    app.run().unwrap();
}
