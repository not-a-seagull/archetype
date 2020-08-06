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

pub struct SelectGuiMode {
    mode: SelectionMode,
    mouse_click_alternator: bool,
}

impl SelectGuiMode {
    #[inline]
    pub const fn new() -> Self {
        Self {
            mode: SelectionMode::NoSelection,
            mouse_click_alternator: false,
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
            'u' => {
                self.mode = SelectionMode::NoSelection;
                gui.project().write().current_frame_mut().unselect();
                gui.update_image();
            }
            'd' => {
                self.mode = SelectionMode::NoSelection;
                gui.project().write().current_frame_mut().delete_selected();
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

    #[inline]
    fn mouse_press(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        if btn == 1 {
            if self.mouse_click_alternator {
                self.mode = SelectionMode::NearestPt;
                gui.project()
                    .write()
                    .current_frame_mut()
                    .select_closest_element(pt);
                gui.update_image();
                self.mouse_click_alternator = false;
            } else {
                self.mouse_click_alternator = true;
            }
        }
    }
}
