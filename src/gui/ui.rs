// GPL v3.0

use super::{Gui, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use gio::{prelude::*, subclass::prelude::*};
use glib::{
    subclass::{self, prelude::*},
    Object,
};
use gtk::{
    prelude::*, subclass::prelude::*, Align, Application, ApplicationWindow, Box as GtkBox,
    DrawingArea, DrawingAreaBuilder, Fixed, Image as GtkImage, Inhibit, Orientation, Widget,
};

pub fn build_ui(application: &Application, gui: Gui) {
    let window = ApplicationWindow::new(application);
    let gtk_box = GtkBox::new(Orientation::Vertical, 1);
    let (width, height) = gui.dimensions();
//    let drawing_area = DrawingAreaBuilder::new()
//        .halign(Align::Start)
//        .valign(Align::Start)
//        .hexpand(false)
//        .vexpand(false)
//        .build();
    let drawing_area = DrawingArea::new();
    drawing_area.set_size_request(width as i32, height as i32);

    // set the drawing area in the GUI widget
    gui.set_drawing_area(drawing_area);

    let g = gui.clone();
    gui.set_drawing_function(move |_da, c| {
        g.clone().draw(c);
        Inhibit(false)
    });

    window.set_default_size(width as i32, height as i32);
    gtk_box.pack_start(&*gui.drawing_area(), true, true, 1);
    window.add(&gtk_box);
    window.show_all();
}
