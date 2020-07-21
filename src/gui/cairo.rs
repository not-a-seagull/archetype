// GPL v3.0

use std::{ffi::c_void, os::raw::c_int};

#[repr(C)]
pub struct cairo_surface_t(c_void);

// extern functions we need to be able to link
extern "C" {
    pub fn cairo_image_surface_get_data(s: *mut cairo_surface_t) -> *mut u8;
    pub fn cairo_image_surface_get_stride(s: *mut cairo_surface_t) -> c_int;
}
