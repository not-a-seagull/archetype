// GPL v3.0

use super::{Gui, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use cairo::{prelude::*, Content, Context, Surface};
use gio::prelude::*;
use gtk::{prelude::*, Application, ApplicationWindow, DrawingArea};
use std::boxed::Box;

pub fn build_ui(app: &Application, gui: &mut Gui) {
    let window = ApplicationWindow::new(application);
    let drawing_area = DrawingArea::new();

    // set the drawing area in the GUI widget
    gui.set_drawing_area(drawing_area);

    let gui_clone = gui.clone();

    // connect up signals
    gui.drawing_area().connect_draw(move |dr, c| {
        // draw an image onto the plot
        let gui_clone_clone = gui_clone.clone();
        gui_clone_clone.draw(c);
    });

    window.set_default_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
    window.add(gui.drawing_area());
    window.show_all();
}
