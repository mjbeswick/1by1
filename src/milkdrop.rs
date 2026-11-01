//! Offscreen Milkdrop renderer for the bottom-left panel.

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::PathBuf;

use fltk::enums::ColorDepth;
use fltk::image::{RgbImage, SharedImage};

unsafe extern "C" {
    fn md_create(w: c_int, h: c_int, preset_dir: *const c_char) -> *mut c_void;
    fn md_resize(ptr: *mut c_void, w: c_int, h: c_int);
    fn md_pcm(ptr: *mut c_void, stereo: *const f32, frames: c_int);
    fn md_render(ptr: *mut c_void);
    fn md_pixels(ptr: *mut c_void) -> *const u8;
    fn md_next(ptr: *mut c_void);
    fn md_preset_count(ptr: *mut c_void) -> u32;
    fn md_preset_name(ptr: *mut c_void, index: u32, out: *mut c_char, cap: c_int) -> c_int;
    fn md_current_preset(ptr: *mut c_void) -> u32;
    fn md_select_preset(ptr: *mut c_void, index: u32);
    fn md_destroy(ptr: *mut c_void);
}

pub struct Milkdrop {
    ptr: *mut c_void,
    w: i32,
    h: i32,
}

impl Milkdrop {
    pub fn new(w: i32, h: i32) -> Option<Self> {
        let dir = preset_dir()?;
        let dir = CString::new(dir.to_string_lossy().as_bytes()).ok()?;
        let ptr = unsafe { md_create(w.max(16), h.max(16), dir.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        Some(Self {
            ptr,
            w: w.max(16),
            h: h.max(16),
        })
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        let (w, h) = (w.max(16), h.max(16));
        if w == self.w && h == self.h {
            return;
        }
        unsafe { md_resize(self.ptr, w, h) };
        self.w = w;
        self.h = h;
    }

    pub fn feed(&mut self, mono: &[f32]) {
        if mono.is_empty() {
            return;
        }
        let mut stereo = Vec::with_capacity(mono.len() * 2);
        for &v in mono {
            stereo.push(v);
            stereo.push(v);
        }
        unsafe { md_pcm(self.ptr, stereo.as_ptr(), mono.len() as c_int) };
    }

    pub fn next_preset(&mut self) {
        unsafe { md_next(self.ptr) };
    }

    pub fn preset_count(&self) -> u32 {
        unsafe { md_preset_count(self.ptr) }
    }

    pub fn preset_name(&self, index: u32) -> String {
        let mut buf = [0 as c_char; 512];
        let ok = unsafe { md_preset_name(self.ptr, index, buf.as_mut_ptr(), buf.len() as c_int) };
        if ok == 0 {
            return String::new();
        }
        unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn preset_names(&self) -> Vec<String> {
        (0..self.preset_count())
            .map(|i| self.preset_name(i))
            .collect()
    }

    pub fn current_preset(&self) -> u32 {
        unsafe { md_current_preset(self.ptr) }
    }

    pub fn select_preset(&mut self, index: u32) {
        unsafe { md_select_preset(self.ptr, index) };
    }

    pub fn render(&mut self) -> Option<SharedImage> {
        unsafe { md_render(self.ptr) };
        let ptr = unsafe { md_pixels(self.ptr) };
        if ptr.is_null() {
            return None;
        }
        let len = self.w as usize * self.h as usize * 4;
        let rgba = unsafe { std::slice::from_raw_parts(ptr, len) };
        let img = RgbImage::new(rgba, self.w, self.h, ColorDepth::Rgba8).ok()?;
        SharedImage::from_image(&img).ok()
    }

    /// Latest frame as raw RGBA, for upload to a GL texture.
    pub fn frame_rgba(&mut self) -> Option<(Vec<u8>, i32, i32)> {
        unsafe { md_render(self.ptr) };
        let ptr = unsafe { md_pixels(self.ptr) };
        if ptr.is_null() || self.w < 1 || self.h < 1 {
            return None;
        }
        let len = self.w as usize * self.h as usize * 4;
        let rgba = unsafe { std::slice::from_raw_parts(ptr, len) };
        Some((rgba.to_vec(), self.w, self.h))
    }
}

impl Drop for Milkdrop {
    fn drop(&mut self) {
        unsafe { md_destroy(self.ptr) };
    }
}

fn preset_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var("ONEBY1_PRESETS") {
        candidates.push(PathBuf::from(p));
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".config/1by1/presets"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("../Resources/presets"));
        }
    }
    candidates.push(PathBuf::from("/opt/homebrew/share/projectM/presets"));
    candidates.push(PathBuf::from("/usr/local/share/projectM/presets"));
    candidates.into_iter().find(|p| p.is_dir())
}
