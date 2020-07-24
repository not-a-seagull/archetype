// GPL v3.0

use super::{Gui, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use euclid::point2;
use gio::{prelude::*, subclass::prelude::*};
use glib::{
    subclass::{self, prelude::*},
    Object,
};
use gtk::{
    prelude::*, subclass::prelude::*, Align, Application, ApplicationWindow, Box as GtkBox,
    DrawingArea, DrawingAreaBuilder, Fixed, Image as GtkImage, Inhibit, Orientation, Widget,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
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
        if evb.get_button() == 1 {
            //            println!("Starting drag...");

            // check if there is currently a line drag (there should not be one)
            let gc = g2.clone();
            let mut line = gc.drag_line().write();
            if line.is_none() {
                let pt: (f32, f32) = match evb.get_position() {
                    (a, b) => (a as f32, b as f32),
                };
                *line = Some((pt.into(), pt.into()));
            }
        }

        Inhibit(false)
    });
    let g3 = gui.clone();
    window.connect_motion_notify_event(move |_da, evm| {
        //        println!("Mouse motion: {:?}", evm.get_position());

        let gc = g3.clone();
        let line = RwLock::upgradable_read(gc.drag_line());
        if line.is_some() {
            let mut line = RwLockUpgradableReadGuard::upgrade(line);
            let pt: (f32, f32) = match evm.get_position() {
                (a, b) => (a as f32, b as f32),
            };
            line.as_mut().unwrap().1 = pt.into();
            mem::drop(line);
            gc.drawing_area().queue_draw();
        }

        Inhibit(false)
    });
    let g4 = gui.clone();
    window.connect_button_release_event(move |_da, evb| {
        if evb.get_button() == 1 {
            let gc = g4.clone();
            let mut line = gc.drag_line().write();
            let (pt1, pt2) = match line.take() {
                Some((pt1, pt2)) => (pt1, pt2),
                None => return Inhibit(false),
            };

            gc.project()
                .write()
                .current_frame_mut()
                .add_buffered_line(pt1, pt2);
            gc.update_image();
        }

        Inhibit(false)
    });
    let g5 = gui.clone();
    window.connect_key_press_event(move |_w, evk| {
        let gc = g5.clone();
        match evk.get_keyval().to_unicode() {
            Some('d') => {
                gc.project()
                    .write()
                    .current_frame_mut()
                    .drop_buffered_lines();
                gc.update_image();
            }
            Some('b') => {
                let mut pr = gc.project().write();
                let brush = pr.current_brush_index();
                pr.current_frame_mut().bezierify_buffered_lines(brush);
                mem::drop(pr);
                gc.update_image();
            }
            _ => (),
        }
        Inhibit(false)
    });

    window.set_default_size(width as i32, height as i32);
    window.set_resizable(false);
    gtk_box.pack_start(&*dr, true, true, 1);
    window.add(&gtk_box);
    window.show_all();
}
