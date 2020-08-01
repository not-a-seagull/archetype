// GPLv3 License

use super::{Gui, Project};
use cairo::Context;
use euclid::default::Point2D;
use parking_lot::{Mutex, RwLock};
use pathfinder_geometry::vector::Vector2F;
use std::mem;

mod buffered;
pub use buffered::{BufferedGuiMode, DEFAULT_ERROR};
mod freedraw;
pub use freedraw::*;
mod select;
pub use select::*;

/// Various modes of GUI
pub trait GuiMode {
    /// Handle a key press.
    fn key_press(&mut self, c: char, gui: &Gui);
    /// Handle mouse press.
    #[inline]
    fn mouse_press(&mut self, _btn: u32, _pt: Vector2F, _gui: &Gui) {}
    /// Handle mouse release.
    #[inline]
    fn mouse_release(&mut self, _btn: u32, _pt: Vector2F, _gui: &Gui) {}
    /// Handle mouse motion.
    #[inline]
    fn mouse_move(&mut self, _pt: Vector2F, _gui: &Gui) {}
    /// Handle being switched in.
    fn switch_in(&mut self, gui: &Gui);
    /// Handle being switched out.
    fn switch_out(&mut self, gui: &Gui);
    /// Handle drawing
    #[inline]
    fn draw(&mut self, _gui: &Gui, _context: &Context) {}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum GuiModeType {
    Switching,
    Buffered,
    Freedraw,
    Select,
}

pub enum GuiModeStorage {
    Switching,
    Buffered(BufferedGuiMode),
    Freedraw(FreedrawGuiMode),
    Select(SelectGuiMode),
}

impl GuiModeStorage {
    #[inline]
    pub fn generic_mut(&mut self) -> Option<&mut dyn GuiMode> {
        Some(match self {
            Self::Switching => return None,
            Self::Buffered(ref mut b) => b,
            Self::Freedraw(ref mut f) => f,
            Self::Select(ref mut s) => s,
        })
    }

    #[inline]
    pub fn kind(&self) -> GuiModeType {
        match self {
            Self::Switching => GuiModeType::Switching,
            Self::Buffered(_) => GuiModeType::Buffered,
            Self::Freedraw(_) => GuiModeType::Freedraw,
            Self::Select(_) => GuiModeType::Select,
        }
    }
}

impl GuiMode for GuiModeStorage {
    #[inline]
    fn switch_in(&mut self, gui: &Gui) {
        self.generic_mut().map(|m| m.switch_in(gui));
    }

    #[inline]
    fn switch_out(&mut self, gui: &Gui) {
        self.generic_mut().map(|m| m.switch_out(gui));
    }

    #[inline]
    fn key_press(&mut self, c: char, gui: &Gui) {
        match self.generic_mut() {
            Some(m) => m.key_press(c, gui),
            None => {
                // see which mode to switch into
                let mut new_mode: GuiModeStorage = match c {
                    'b' => gui
                        .take_matching_gui_mode(GuiModeType::Buffered)
                        .unwrap_or_else(|| Self::Buffered(BufferedGuiMode::new(DEFAULT_ERROR))),
                    'f' => gui
                        .take_matching_gui_mode(GuiModeType::Freedraw)
                        .unwrap_or_else(|| Self::Freedraw(FreedrawGuiMode::new())),
                    's' => gui
                        .take_matching_gui_mode(GuiModeType::Select)
                        .unwrap_or_else(|| Self::Select(SelectGuiMode::new())),
                    _ => GuiModeStorage::Switching,
                };

                new_mode.switch_in(gui);
                *self = new_mode;
            }
        }
    }

    #[inline]
    fn mouse_press(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        self.generic_mut().map(|m| m.mouse_press(btn, pt, gui));
    }

    #[inline]
    fn mouse_release(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        self.generic_mut().map(|m| m.mouse_release(btn, pt, gui));
    }

    #[inline]
    fn mouse_move(&mut self, pt: Vector2F, gui: &Gui) {
        self.generic_mut().map(|m| m.mouse_move(pt, gui));
    }

    #[inline]
    fn draw(&mut self, gui: &Gui, context: &Context) {
        self.generic_mut().map(|m| m.draw(gui, context));
    }
}
