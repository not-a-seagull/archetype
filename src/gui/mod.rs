// GPL v3.0

use super::{Color, colors, render, AlphaMaskTarget, Brush, GraphicalState, RenderTarget};
use cairo::{Context, Format, ImageSurface};
use euclid::default::Point2D;
use gio::{prelude::*, ApplicationFlags};
use glib::Bytes;
use gtk::{prelude::*, Application, ApplicationWindow, DrawingArea, Image as GtkImage};
use image::{Rgba, RgbaImage};
use once_cell::sync::OnceCell;
use parking_lot::{
    MappedMutexGuard, MappedRwLockReadGuard, Mutex, MutexGuard, RwLock, RwLockReadGuard,
    RwLockUpgradableReadGuard, RwLockWriteGuard,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    env,
    fs::File,
    io::{self, prelude::*},
    mem,
    sync::Arc,
};

mod mode;
mod ui;

pub const DEFAULT_WIDTH: u32 = 300;
pub const DEFAULT_HEIGHT: u32 = 200;

pub use mode::*;

#[inline]
fn standard_brushes() -> SmallVec<[Brush; 10]> {
    let mut sm = SmallVec::new();
    sm.push(Brush::new(colors::BLACK, 2));
    sm
}

#[derive(Serialize, Deserialize, Copy, Clone)]
enum ProjectSave {
    Bincode,
    Json,
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
    filename: Option<String>,
    filetype: Option<ProjectSave>,
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

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
}

struct GuiInternal {
    current_project: RwLock<Project>,
    image: RwLock<(RgbaImage, bool)>, // the bool is a flag to tell if it has been modified
    application: Application,

    canvas: OnceCell<DrawingArea>,
    main_window: OnceCell<ApplicationWindow>,
    surface: Mutex<Option<ImageSurface>>,

    gui_mode: Mutex<GuiModeStorage>,
    past_gui_modes: Mutex<SmallVec<[GuiModeStorage; 5]>>,
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Gui(Arc<GuiInternal>);

const DEFAULT_ERROR: f32 = 12.0;

impl Gui {
    pub fn new(project: Project) -> Gui {
        let application = Application::new(
            Some("com.notaseagull.archetype"),
            ApplicationFlags::HANDLES_COMMAND_LINE,
        )
        .expect("Unable to initialize GTK");

        let img = RgbaImage::from_pixel(project.width, project.height, Rgba([255, 255, 255, 0]));
        let mut gui = Self(Arc::new(GuiInternal {
            current_project: RwLock::new(project),
            application,
            canvas: OnceCell::new(),
            main_window: OnceCell::new(),
            image: RwLock::new((img, true)),
            surface: Mutex::new(None),
            gui_mode: Mutex::new(GuiModeStorage::Buffered(BufferedGuiMode::new(
                DEFAULT_ERROR,
            ))),
            past_gui_modes: Mutex::new(SmallVec::new()),
        }));

        let cl = gui.clone();
        gui.0.application.connect_command_line(|app, _cmd| {
            app.activate();
            0
        });
        gui.0.application.connect_activate(move |app| {
            ui::build_ui(app, cl.clone());
            cl.update_image();
        });

        gui
    }

    #[inline]
    pub fn new_project(width: u32, height: u32) -> Gui {
        let project = Project {
            width,
            height,
            frames: vec![GraphicalState::new()],
            brushes: standard_brushes(),
            current_frame: 0,
            current_brush: 0,
            filename: None,
            filetype: None,
        };
        Self::new(project)
    }

    #[inline]
    pub fn gui_mode(&self) -> &Mutex<GuiModeStorage> {
        &self.0.gui_mode
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
        frame.rasterize(&self.0.image, &*pr);
        self.0
            .canvas
            .get()
            .expect("Canvas does not yet exist")
            .queue_draw();
    }

    #[inline]
    pub fn project(&self) -> &RwLock<Project> {
        &self.0.current_project
    }

    #[inline]
    pub fn set_drawing_area(&self, dr: DrawingArea) {
        self.0.canvas.set(dr).unwrap();
    }

    #[inline]
    pub fn set_main_window(&self, mw: ApplicationWindow) {
        self.0.main_window.set(mw).unwrap();
    }

    #[inline]
    pub fn set_drawing_function<F>(&self, fnd: F)
    where
        F: Fn(&DrawingArea, &Context) -> Inhibit + 'static,
    {
        self.drawing_area().connect_draw(fnd);
    }

    #[inline]
    pub fn drawing_area(&self) -> &DrawingArea {
        self.0.canvas.get().expect("Drawing area does not exist")
    }

    #[inline]
    pub fn main_window(&self) -> &ApplicationWindow {
        self.0
            .main_window
            .get()
            .expect("Application window does not exist")
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

            self.gui_mode().lock().draw(self, context);
        }
    }

    #[inline]
    pub fn hide(&self) {
        self.main_window().hide();
    }

    #[inline]
    pub fn show(&self) {
        self.main_window().show_all();
    }

    #[inline]
    pub fn save_project(&self, force_rename: bool) -> Result<(), &'static str> {
        let pr = RwLock::upgradable_read(&self.0.current_project);
        let pr = if pr.filename.is_none() || force_rename {
            let mut pr = RwLockUpgradableReadGuard::upgrade(pr);
            self.hide();

            let si = io::stdin();
            let so = io::stdout();
            let mut stdin = si.lock();
            let mut stdout = so.lock();

            let mut filename = String::new();
            let mut save_type = String::new();

            stdout.write_all(b"Enter file name: ").unwrap();
            stdout.flush().unwrap();
            stdin.read_line(&mut filename).unwrap();

            filename.pop();
            pr.filename = Some(filename);

            'stype: loop {
                stdout
                    .write_all(b"Save as bincode (b) or JSON (j): ")
                    .unwrap();
                stdout.flush().unwrap();
                stdin.read_line(&mut save_type).unwrap();

                match save_type.remove(0) {
                    'b' => {
                        pr.filetype = Some(ProjectSave::Bincode);
                        break 'stype;
                    }
                    'j' => {
                        pr.filetype = Some(ProjectSave::Json);
                        break 'stype;
                    }
                    _ => (),
                }
            }

            self.show();
            RwLockWriteGuard::downgrade(pr)
        } else {
            RwLockUpgradableReadGuard::downgrade(pr)
        };

        // open the file for writing
        let mut f =
            File::create(pr.filename.as_ref().unwrap()).map_err(|_e| "Unable to open file")?;

        match pr.filetype {
            Some(ProjectSave::Bincode) => {
                // use the bincode serializer to serialize the file to bytes
                let bytes =
                    bincode::serialize(&*pr).map_err(|_e| "Unable to serialize to bytes")?;
                f.write_all(&bytes)
                    .map_err(|_e| "Unable to write to file")?;
            }
            Some(ProjectSave::Json) => {
                let json =
                    serde_json::to_string(&*pr).map_err(|_e| "Unable to serialize to JSON")?;
                f.write_all(json.as_bytes())
                    .map_err(|_e| "Unable to write to file")?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    #[inline]
    pub fn export_project(&self) -> Result<(), &'static str> {
        let si = io::stdin();
        let so = io::stdout();
        let mut stdin = si.lock();
        let mut stdout = so.lock();

        let mut filename = String::new();
        let mut outtype_raw = String::new();
        let mut outtype: Option<RenderTarget> = None;
        let mut yn_alpha = String::new();

        stdout.write_all(b"Enter export filename: ").unwrap();
        stdout.flush().unwrap();
        stdin.read_line(&mut filename).unwrap();

        filename.pop();

        const PROMPT: &'static [u8] = b"
The following export file types are supported:
 * (s)ingle image: Export the currently selected frame as a PNG image.
 * (m)p4 video

Enter format: ";

        while outtype.is_none() {
            stdout.write_all(PROMPT).unwrap();
            stdout.flush().unwrap();
            stdin.read_line(&mut outtype_raw).unwrap();

            outtype = RenderTarget::from_char(outtype_raw.remove(0));
        }

        mem::drop(stdin);
        mem::drop(stdout);

        let mut alphaname = filename.clone();
        let am = if outtype.unwrap().is_single_image() {
            AlphaMaskTarget::Background(unsafe { Color::new_unchecked(0.0, 0.0, 0.0) })
        } else if crate::interactive_yn("Export an alpha mask alongside the final product?") {
            alphaname.push_str(".alpha");
            AlphaMaskTarget::AlphaMask(&alphaname)
        } else {
            AlphaMaskTarget::Background(crate::interactive_color("background color"))
        };

        render(
            &*self.0.current_project.read(),
            &filename,
            outtype.unwrap(),
            am,
        )
    }
}
