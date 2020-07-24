// GPL v3.0

use super::{colors, Brush, GraphicalState};
use cairo::{Context, Format, ImageSurface};
use euclid::default::Point2D;
use gio::prelude::*;
use glib::Bytes;
use gtk::{prelude::*, Application, DrawingArea, Image as GtkImage};
use image::{Rgba, RgbaImage};
use parking_lot::{
    MappedMutexGuard, MappedRwLockReadGuard, Mutex, MutexGuard, RwLock, RwLockReadGuard,
    RwLockUpgradableReadGuard, RwLockWriteGuard,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{env, mem, sync::Arc};

mod ui;

pub const DEFAULT_WIDTH: u32 = 300;
pub const DEFAULT_HEIGHT: u32 = 200;

#[inline]
fn standard_brushes() -> SmallVec<[Brush; 10]> {
    let mut sm = SmallVec::new();
    sm.push(Brush::new(colors::BLACK, 2));
    sm
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    // the graphical frames contained within
    width: u32,
    height: u32,
    brushes: SmallVec<[Brush; 10]>,
    frames: Vec<GraphicalState>,
    current_frame: usize,
    current_brush: usize,
}

impl Project {
    #[inline]
    pub fn current_frame(&self) -> &GraphicalState {
        &self.frames[self.current_frame]
    }

    #[inline]
    pub fn current_frame_mut(&mut self) -> &mut GraphicalState {
        &mut self.frames[self.current_frame]
    }

    #[inline]
    pub fn brush(&self, index: usize) -> Option<&Brush> {
        self.brushes.get(index)
    }

    #[inline]
    pub fn current_brush(&self) -> &Brush {
        &self.brushes[self.current_brush]
    }

    #[inline]
    pub fn current_brush_index(&self) -> usize {
        self.current_brush
    }
}

struct GuiInternal {
    current_project: RwLock<Project>,
    image: RwLock<(RgbaImage, bool)>, // the bool is a flag to tell if it has been modified
    application: Application,
    canvas: RwLock<Option<DrawingArea>>,
    surface: Mutex<Option<ImageSurface>>,

    // the current drag starting point
    drag_line: RwLock<Option<(Point2D<f32>, Point2D<f32>)>>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Gui(Arc<GuiInternal>);

const BYTES_PER_PIXEL: i32 = 4;

impl Gui {
    pub fn new(project: Project) -> Gui {
        let application = Application::new(Some("com.notaseagull.archetype"), Default::default())
            .expect("Unable to initialize GTK");
        let img = RgbaImage::from_pixel(project.width, project.height, Rgba([255, 255, 255, 0]));
        let mut gui = Self(Arc::new(GuiInternal {
            current_project: RwLock::new(project),
            application,
            canvas: RwLock::new(None),
            image: RwLock::new((img, true)),
            surface: Mutex::new(None),
            drag_line: RwLock::new(None),
        }));

        let cl = gui.clone();
        gui.0.application.connect_activate(move |app| {
            ui::build_ui(app, cl.clone());
            cl.update_image();
        });

        gui
    }

    pub fn new_project(width: u32, height: u32) -> Gui {
        let project = Project {
            width,
            height,
            frames: vec![GraphicalState::new()],
            brushes: standard_brushes(),
            current_frame: 0,
            current_brush: 0,
        };
        Self::new(project)
    }

    #[inline]
    pub fn update_image(&self) {
        // wipe the image
        self.0
            .image
            .write()
            .0
            .as_flat_samples_mut()
            .image_mut_slice()
            .unwrap()
            .par_iter_mut()
            .for_each(|m| *m = 0);

        let pr = self.0.current_project.read();
        let frame = &pr.frames[pr.current_frame];
        frame.rasterize(&self.0.image, &self.0.current_project);
        self.0
            .canvas
            .read()
            .as_ref()
            .expect("Canvas does not yet exist")
            .queue_draw();
    }

    #[inline]
    pub fn drag_line(&self) -> &RwLock<Option<(Point2D<f32>, Point2D<f32>)>> {
        &self.0.drag_line
    }

    #[inline]
    pub fn project(&self) -> &RwLock<Project> {
        &self.0.current_project
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
                            [_, _, _, 0] => [219, 252, 255, 255],
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

            context.set_source_surface(&*surface, 0.0 as f64, 0.0 as f64);
            context.paint();

            // draw a red line on top of it if we need to
            let drag_line = self.0.drag_line.read();
            if let Some(ref drag_line) = &*drag_line {
                context.set_source_rgb(1.0, 0.0, 0.0);
                context.set_line_width(100.0);
                context.move_to((drag_line.0).x.into(), (drag_line.0).y.into());
                context.line_to((drag_line.1).x.into(), (drag_line.1).y.into());
            }
        }
    }
}
