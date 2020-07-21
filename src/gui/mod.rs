// GPL v3.0

use super::GraphicalState;
use cairo::{prelude::*, Content, Context, Surface};
use gtk::{prelude::*, Application, DrawingArea};
use image::{Rgba, RgbaImage};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{env, mem, sync::Arc};

mod cairo;
mod ui;

pub const DEFAULT_WIDTH: u32 = 300;
pub const DEFAULT_HEIGHT: u32 = 200;

#[derive(Serialize, Deserialize)]
pub struct Project {
    // the graphical frames contained within
    width: u32,
    height: u32,
    frames: Vec<GraphicalState>,
    current_frame: usize,
}

struct GuiInternal {
    current_project: RwLock<Option<Project>>,
    image: RwLock<RgbaImage>,
    application: Application,
    canvas: Option<DrawingArea>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Gui(Arc<GuiInternal>);

impl Gui {
    pub fn new() -> Gui {
        let application = Application::new(Some("com.notaseagull.archetype"), Default::default())
            .expect("Unable to initialize GTK");
        let mut gui = Self(Arc::new(GuiInternal {
            current_project: RwLock::new(None),
            image: RwLock::new(RgbaImage::from_pixel(Rgba(0, 0, 0, 0))),
            application,
            canvas: None,
            surface: Mutex::new(None),
        }));

        application.connect_activate(|app| {
            ui::build_ui(app, &mut gui);
        });

        gui
    }

    #[inline]
    pub fn drawing_area(&self) -> &DrawingArea {
        &self.0.canvas
    }

    #[inline]
    pub fn set_drawing_area(&mut self, dr: DrawingArea) {
        self.0
            .get_mut()
            .expect("More than one reference should not have been created")
            .canvas = Some(dr);
    }

    #[inline]
    pub fn run(&self) {
        self.application.run(&env::args().collect::<Vec<_>>());
    }

    pub fn draw(&self, context: &Context) {
        let surface = context.get_target();

        // we need to get the raw C object for the data
        surface.flush();
        let raw_surface = surface.to_raw_none() as *mut cairo::cairo_surface_t;
        let mut data = cairo::cairo_image_surface_get_data(raw_surface);
        let stride = cairo::cairo_image_surface_get_stride(raw_surface);

        // get the image width and height
        let img = self.0.image.read();
        let width = img.width();
        let height = img.height();

        // draw pixels into data
        img.enumerate_rows().fold(data, |data, (y, row)| {
            let mut row = data as *mut u32;
            row.for_each(|(x, _y, pixel)| {
                let (r, g, b) = match pixel.0[3] {
                    std::u8::MAX => (pixel.0[0], pixel.0[1], pixel.0[2]),
                    _ => (0, 0, 255),
                };

                let val = (r << 16) | (g << 8) | b;
                unsafe { row.offset(x).write(val) };
            });

            // move the data up by a stride
            data.offset(stride)
        });

        // mark the surface as dirty
        surface.mark_dirty();
    }
}
