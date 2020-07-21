// GPL v3.0

#![allow(incomplete_features)]
#![feature(const_generics)]

mod bezier;
mod brush;
mod gui;
mod polynomial;
mod state;

pub use bezier::*;
pub use brush::*;
pub use polynomial::*;
pub use state::*;

fn main() {
    println!("Hello, world!");
}
