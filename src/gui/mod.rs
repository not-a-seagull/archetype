// GPL v3.0

use super::GraphicalState;
use cairo::{Context, Format, ImageSurface};
use gio::prelude::*;
use glib::Bytes;
use gtk::{prelude::*, Application, DrawingArea, Image as GtkImage};
use image::{Rgba, RgbaImage};
use parking_lot::{
    MappedMutexGuard, MappedRwLockReadGuard, Mutex, MutexGuard, RwLock, RwLockReadGuard,
    RwLockUpgradableReadGuard, RwLockWriteGuard,
};
use serde::{Deserialize, Serialize};
use std::{env, mem, sync::Arc};

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
    current_project: RwLock<Project>,
    image: RwLock<(RgbaImage, bool)>, // the bool is a flag to tell if it has been modified
    application: Application,
    canvas: RwLock<Option<DrawingArea>>,
    surface: Mutex<Option<ImageSurface>>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Gui(Arc<GuiInternal>);

const BYTES_PER_PIXEL: i32 = 4;

impl Gui {
    pub fn new(project: Project) -> Gui {
        let application = Application::new(Some("com.notaseagull.archetype"), Default::default())
            .expect("Unable to initialize GTK");
        let img = RgbaImage::from_fn(project.width, project.height, |x, y| {
            let r = (x as f32 / project.width as f32) * std::u8::MAX as f32;
            let g = (y as f32 / project.height as f32) * std::u8::MAX as f32;
            let b = 200;
            let a = std::u8::MAX;
            Rgba([r as u8, g as u8, b, a])
        });
        let mut gui = Self(Arc::new(GuiInternal {
            current_project: RwLock::new(project),
            application,
            canvas: RwLock::new(None),
            image: RwLock::new((img, true)),
            surface: Mutex::new(None),
        }));

        let cl = gui.clone();
        gui.0.application.connect_activate(move |app| {
            ui::build_ui(app, cl.clone());
        });

        gui
    }

    pub fn new_project(width: u32, height: u32) -> Gui {
        let project = Project {
            width,
            height,
            frames: vec![GraphicalState::new()],
            current_frame: 0,
        };
        Self::new(project)
    }

    #[inline]
    pub fn set_drawing_area(&self, dr: DrawingArea) {
        *self.0.canvas.write() = Some(dr);
    }

    #[inline]
    pub fn set_drawing_function<F>(&self, fnd: F)
    where
        F: Fn(&DrawingArea, &Context) -> Inhibit + 'static,
    {
        self.drawing_area().connect_draw(fnd);
    }

    #[inline]
    pub fn drawing_area(&self) -> MappedRwLockReadGuard<'_, DrawingArea> {
        RwLockReadGuard::map(self.0.canvas.read(), |c| match c {
            None => panic!("Drawing area does not exist"),
            Some(ref dr) => dr,
        })
    }

    #[inline]
    pub fn run(&self) {
        self.0.application.run(&env::args().collect::<Vec<_>>());
    }

    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        let pr = self.0.current_project.read();
        (pr.width, pr.height)
    }

    #[inline]
    pub fn update_image(&self) {
        let mut img = self.0.image.write(); // acquire write lock
        img.1 = true;
    }

    // rebuild the surface
    #[inline]
    pub fn recreate_surface(&self) {
        let mut surface = self.0.surface.lock();

        // clone the data from the image
        let img = self.0.image.read();
        let data: Box<[u8]> = img
            .0
            .as_flat_samples()
            .image_slice()
            .expect("Unable to get image data")
            .into();

        *surface = Some(
            ImageSurface::create_for_data(
                data,
                Format::Rgb24,
                img.0.width() as i32,
                img.0.height() as i32,
                4i32 * img.0.width() as i32,
            )
            .expect("Unable to create surface"),
        );
    }

    pub fn draw(&self, context: &Context) {
        let surface = self.0.surface.lock();
        if surface.is_none() {
            mem::drop(surface); // free up the mutex
            self.recreate_surface();
            // do a recursive call on this function
            self.draw(context);
        } else {
            let mut surface = MutexGuard::map(surface, |s| s.as_mut().unwrap());

            surface.flush();
            let img = RwLock::upgradable_read(&self.0.image);

            // get the image width and height
            let width = img.0.width();
            let height = img.0.height();

            if img.1 {
                // the write flag is toggled to on
                let mut img = RwLockUpgradableReadGuard::upgrade(img);
                img.1 = false;
                // downgrade to a read lock
                let img = RwLockWriteGuard::downgrade(img);

                let stride = surface.get_stride() as usize;
                let mut data = surface.get_data().expect("Unable to borrow surface data");

                // draw pixels into data
                img.0.enumerate_rows().fold(&mut *data, |data, (y, row)| {
                    row.for_each(|(x, _y, pixel)| {
                        let pixel: [u8; 4] = match pixel.0 {
                            //                            [_, _, _, 0] => [219, 252, 255, 255],
                            [r, g, b, a] => [b, g, r, a],
                        };

                        data.iter_mut()
                            .skip(x as usize * 4)
                            .take(4)
                            .zip(pixel.iter())
                            .for_each(|(b, val)| *b = *val);
                    });

                    // move the data up by a stride
                    &mut data[stride..]
                });

                mem::drop(data);

                // mark the surface as dirty
                surface.mark_dirty();
                mem::drop(img);
            } else {
                // don't hog the write lock
                mem::drop(img);
            }

            context.set_source_surface(&*surface, width as f64, height as f64);
            context.paint();
        }
    }
}
