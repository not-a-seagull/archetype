// GPL v3.0

#![allow(incomplete_features)]
#![feature(const_generics)]

mod bezier;
mod brush;
mod drawing;
mod geometry;
mod gui;
mod interactive;
mod polygon;
mod render;
mod state;

pub use bezier::*;
pub use brush::*;
pub use drawing::*;
pub use geometry::*;
pub use gui::*;
pub use interactive::*;
pub use polygon::*;
pub use render::*;
pub use state::*;

use image::{ImageBuffer, Rgba};
use parking_lot::RwLock;
use std::{
    env,
    fs::File,
    io::{self, prelude::*},
    mem, thread,
    time::Duration,
};

/// A true-color RGBA image.
pub type TCImage = ImageBuffer<Rgba<u16>, Vec<u16>>;
/// The locked image buffer.
pub type DrawTarget = RwLock<(TCImage, bool)>;

// spawns a quick deadlock detector
fn deadlock_detector() {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(10));

        let deadlocks = parking_lot::deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        println!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            println!("Deadlock #{}", i);
            for t in threads {
                println!("Thread Id {:#?}", t.thread_id());
                println!("{:#?}", t.backtrace());
            }
        }
    });
}

fn main() {
    deadlock_detector();

    let gui = match env::args().nth(1) {
        None => {
            let mut width = String::new();
            let mut height = String::new();

            let so = io::stdout();
            let mut stdout = so.lock();
            let si = io::stdin();
            let mut stdin = si.lock();

            stdout
                .write_all(b"Enter the width of the project: ")
                .unwrap();
            stdout.flush().unwrap();
            stdin.read_line(&mut width).expect("Unable to get width");
            stdout
                .write_all(b"Enter the height of the project: ")
                .unwrap();
            stdout.flush().unwrap();
            stdin.read_line(&mut height).expect("Unable to get height");

            mem::drop(stdin);
            mem::drop(stdout);

            width.pop();
            height.pop();

            let width = width.parse::<u32>().expect("Width is not a number");
            let height = height.parse::<u32>().expect("Height is not a number");

            gui::Gui::new_project(width, height)
        }
        Some(prj_name) => {
            // try to deserialize with bincode
            let mut file = File::open(&prj_name)
                .unwrap_or_else(|_| panic!("Unable to open file \"{}\"", prj_name));
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .expect("Unable to read from file");
            mem::drop(file);

            let project: gui::Project = match bincode::deserialize(&bytes) {
                Ok(prj) => prj,
                Err(_e) => {
                    // if it error'd out, try deserializing from JSON
                    let string =
                        String::from_utf8(bytes).expect("Unable to convert bytes to string");
                    serde_json::from_str(&string)
                        .expect("Unable to deserialize from bincode or json")
                }
            };

            gui::Gui::new(project)
        }
    };
    gui.run();
}
