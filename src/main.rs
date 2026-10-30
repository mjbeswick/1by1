use fltk::{app, button::Button, frame::Frame, group::Flex, prelude::*, window::Window};

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut win = Window::default().with_size(900, 600).with_label("1by1-rs");

    let mut col = Flex::default_fill().column();
    col.set_margin(8);

    // ponytail: transport buttons unwired. Player thread comes next.
    let bar = Flex::default().row();
    for label in ["|<", "Play", "Stop", ">|"] {
        Button::default().with_label(label);
    }
    bar.end();
    col.fixed(&bar, 36);

    // ponytail: placeholder body until library.rs (tree + track list) exists.
    Frame::default().with_label("Folder tree | track list (to do)");

    let status = Frame::default().with_label("Ready");
    col.fixed(&status, 24);

    col.end();
    win.end();
    win.show();
    app.run().unwrap();
}
