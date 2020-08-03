// GPLv3 License

use super::GuiMode;
use crate::Gui;
use euclid::default::Point2D;
use pathfinder_geometry::vector::Vector2F;

enum SelectionMode {
    NoSelection,
    NearestPt,
    History,
}

#[repr(transparent)]
pub struct SelectGuiMode {
    mode: SelectionMode,
}

impl SelectGuiMode {
    #[inline]
    pub const fn new() -> Self {
        Self {
            mode: SelectionMode::NoSelection,
        }
    }
}

impl GuiMode for SelectGuiMode {
    #[inline]
    fn key_press(&mut self, c: char, gui: &Gui) {
        match c {
            'h' => {
                gui.project()
                    .write()
                    .current_frame_mut()
                    .select_from_history();
                self.mode = SelectionMode::History;
                gui.update_image();
            }
            _ => (),
        }
    }

    #[inline]
    fn switch_in(&mut self, _gui: &Gui) {}

    #[inline]
    fn switch_out(&mut self, _gui: &Gui) {
        self.mode = SelectionMode::NoSelection;
    }
}
