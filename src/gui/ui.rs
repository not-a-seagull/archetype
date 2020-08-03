// GPL v3.0

use super::{Gui, GuiMode};
use euclid::point2;
use gtk::{
    prelude::*, subclass::prelude::*, Align, Application, ApplicationWindow, Box as GtkBox,
    DrawingArea, DrawingAreaBuilder, Fixed, Image as GtkImage, Inhibit, Orientation, Widget,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use pathfinder_geometry::vector::Vector2F;

use std::mem;

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

    let dr = gui.drawing_area();
    let g2 = gui.clone();
    window.connect_button_press_event(move |_da, evb| {
        let gc = g2.clone();
        let (x, y) = match evb.get_position() {
            (a, b) => (a as f32, b as f32),
        };

        gc.gui_mode()
            .lock()
            .mouse_press(evb.get_button(), Vector2F::new(x, y), &gc);

        Inhibit(false)
    });
    let g3 = gui.clone();
    window.connect_motion_notify_event(move |_da, evm| {
        //        println!("Mouse motion: {:?}", evm.get_position());

        let gc = g3.clone();
        let (x, y) = match evm.get_position() {
            (a, b) => (a as f32, b as f32),
        };
        gc.gui_mode().lock().mouse_move(Vector2F::new(x, y), &gc);

        Inhibit(false)
    });
    let g4 = gui.clone();
    window.connect_button_release_event(move |_da, evb| {
        let gc = g4.clone();
        let (x, y) = match evb.get_position() {
            (a, b) => (a as f32, b as f32),
        };
        gc.gui_mode()
            .lock()
            .mouse_release(evb.get_button(), Vector2F::new(x, y), &gc);

        Inhibit(false)
    });
    let g5 = gui.clone();
    window.connect_key_press_event(move |_w, evk| {
        use gdk::ModifierType;

        let gc = g5.clone();
        match evk.get_keyval().to_unicode() {
            Some('s') if gc.gui_mode().lock().kind() != super::GuiModeType::Switching => {
                if let Err(e) = gc.save_project(
                    evk.get_state() & ModifierType::SHIFT_MASK != ModifierType::empty(),
                ) {
                    eprintln!("Unable to save file: {}", e);
                }
            }
            Some('e') => {
                if let Err(e) = gc.export_project() {
                    eprintln!("Unable to export file: {}", e);
                }
            }
            Some('m') => {
                // switch into switch mode
                gc.store_gui_mode();
                println!("Activated switch mode");
            }
            Some(c) => gc.gui_mode().lock().key_press(c, &gc),
            _ => (),
        }
        Inhibit(false)
    });

    window.set_default_size(width as i32, height as i32);
    window.set_resizable(false);
    gtk_box.pack_start(&*dr, true, true, 1);
    window.add(&gtk_box);
    window.show_all();
    gui.set_main_window(window);
}
