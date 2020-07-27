// MIT License

use super::{Gui, Project};
use cairo::Context;
use euclid::default::Point2D;
use parking_lot::{Mutex, RwLock};
use pathfinder_geometry::vector::Vector2F;
use std::mem;

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

/// Use the buffered lines.
pub struct BufferedGuiMode {
    drag_line: Option<(Vector2F, Vector2F)>,
    error: f32,
}

impl BufferedGuiMode {
    #[inline]
    pub fn new(error: f32) -> Self {
        Self {
            error,
            drag_line: None,
        }
    }
}

impl GuiMode for BufferedGuiMode {
    #[inline]
    fn switch_in(&mut self, _gui: &Gui) {}

    #[inline]
    fn switch_out(&mut self, _gui: &Gui) {
        self.drag_line = None;
    }

    #[inline]
    fn key_press(&mut self, c: char, gui: &Gui) {
        match c {
            'd' => {
                gui.project()
                    .write()
                    .current_frame_mut()
                    .drop_buffered_lines();
                gui.update_image();
            }
            'b' => {
                let mut pr = gui.project().write();
                let brush = pr.current_brush_index();
                pr.current_frame_mut()
                    .bezierify_buffered_lines(brush, self.error);
                mem::drop(pr);
                gui.update_image();
            }
            'l' => {
                let mut pr = gui.project().write();
                let brush = pr.current_brush_index();
                pr.current_frame_mut().convert_buffered_lines(brush);
                mem::drop(pr);
                gui.update_image();
            }
            'e' => {
                self.error += 1.0;
                println!("Error is {}", self.error);
            }
            'r' => {
                self.error -= 1.0;
                if self.error < 0.0f32 {
                    self.error = 0.1f32;
                }
                println!("Error is {}", self.error);
            }
            _ => (),
        }
    }

    #[inline]
    fn mouse_press(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        if btn == 1 {
            // check if there is currently a line drag (there should not be one)
            if self.drag_line.is_none() {
                self.drag_line = Some((pt, pt));
            }
        }
    }

    #[inline]
    fn mouse_release(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        if let (1, Some((pt1, pt2))) = (btn, self.drag_line.take()) {
            let (pt1, pt2) = (
                Point2D::new(pt1.x(), pt1.y()),
                Point2D::new(pt2.x(), pt2.y()),
            );
            gui.project()
                .write()
                .current_frame_mut()
                .add_buffered_line(pt1, pt2);
            gui.update_image();
        }
    }

    #[inline]
    fn mouse_move(&mut self, pt: Vector2F, gui: &Gui) {
        if let Some((_, ref mut pt2)) = self.drag_line.as_mut() {
            *pt2 = pt;
        }
    }

    #[inline]
    fn draw(&mut self, _gui: &Gui, context: &Context) {
        if let Some(ref drag_line) = &self.drag_line {
            context.set_source_rgb(1.0, 0.0, 0.0);
            context.set_line_width(100.0);
            context.move_to((drag_line.0).x().into(), (drag_line.0).y().into());
            context.line_to((drag_line.1).x().into(), (drag_line.1).y().into());
        }
    }
}

#[derive(Copy, Clone)]
pub enum GuiModeType {
    Buffered,
}

pub enum GuiModeStorage {
    Buffered(BufferedGuiMode),
}

impl GuiModeStorage {
    #[inline]
    pub fn generic_mut(&mut self) -> &mut dyn GuiMode {
        match self {
            Self::Buffered(ref mut b) => b,
        }
    }
}

impl GuiMode for GuiModeStorage {
    #[inline]
    fn switch_in(&mut self, gui: &Gui) {
        self.generic_mut().switch_in(gui);
    }

    #[inline]
    fn switch_out(&mut self, gui: &Gui) {
        self.generic_mut().switch_out(gui);
    }

    #[inline]
    fn key_press(&mut self, c: char, gui: &Gui) {
        self.generic_mut().key_press(c, gui);
    }

    #[inline]
    fn mouse_press(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        self.generic_mut().mouse_press(btn, pt, gui);
    }

    #[inline]
    fn mouse_release(&mut self, btn: u32, pt: Vector2F, gui: &Gui) {
        self.generic_mut().mouse_release(btn, pt, gui);
    }

    #[inline]
    fn mouse_move(&mut self, pt: Vector2F, gui: &Gui) {
        self.generic_mut().mouse_move(pt, gui);
    }

    #[inline]
    fn draw(&mut self, gui: &Gui, context: &Context) {
        self.generic_mut().draw(gui, context);
    }
}
