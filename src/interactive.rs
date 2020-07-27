// MIT License

use super::Color;
use smallvec::SmallVec;
use std::io::{self, prelude::*};

pub fn interactive_yn(prompt: &str) -> bool {
    let si = io::stdin();
    let so = io::stdout();

    let mut stdin = si.lock();
    let mut stdout = so.lock();

    let mut res = String::new();

    'yn: loop {
        stdout
            .write_fmt(format_args!("{} [y/n]: ", prompt))
            .unwrap();
        stdout.flush().unwrap();
        stdin.read_line(&mut res).unwrap();

        if res.len() < 1 {
            continue 'yn;
        }

        match res.remove(0) {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => (),
        }
    }
}

pub fn interactive_color(color_name: &str) -> Color {
    let si = io::stdin();
    let so = io::stdout();

    let mut stdin = si.lock();
    let mut stdout = so.lock();

    let mut res = String::new();

    'clr: loop {
        stdout
            .write_fmt(format_args!("Enter hex code for {}", color_name))
            .unwrap();
        stdout.flush().unwrap();
        stdin.read_line(&mut res).unwrap();

        let iter = res.chars().filter(|c| *c != '#');
        let parts: SmallVec<[Result<u8, _>; 3]> = iter
            .clone()
            .step_by(2)
            .zip(iter.skip(1).step_by(2))
            .map(|(d1, d2)| {
                let hex = [d1, d2].iter().collect::<String>();
                u8::from_str_radix(&hex, 16)
            })
            .collect();

        if let Ok([Ok(r), Ok(g), Ok(b)]) = parts.into_inner() {
            macro_rules! u8tof {
                ($e: expr) => {{
                    $e as f32 / std::u8::MAX as f32
                }};
            }

            return unsafe { Color::new_unchecked(u8tof!(r), u8tof!(g), u8tof!(b)) };
        }
    }
}
