// GPL v3.0

use super::{Gui, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use cairo::{Content, Context, Surface};
use gio::prelude::*;
use gtk::{prelude::*, Application, ApplicationWindow, DrawingArea, Fixed};
use std::boxed::Box;

pub fn build_ui(application: &Application, gui: Gui) {
    let window = ApplicationWindow::new(application);
    let drawing_area = DrawingArea::new();

    // set the drawing area in the GUI widget
    gui.set_drawing_area(drawing_area);

    let gui_clone = gui.clone();

    // connect up signals
    gui.set_drawing_function(move |dr, c| {
        // draw an image onto the plot
        let gui_clone_clone = gui_clone.clone();
        gui_clone_clone.draw(c);
        Inhibit(false)
    });

    window.set_default_size(DEFAULT_WIDTH as i32, DEFAULT_HEIGHT as i32);
    window.add(&*gui.drawing_area());
    window.show_all();
}
