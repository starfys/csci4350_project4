#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int, c_void};

pub type em_arg_callback_func = Option<unsafe extern "C" fn(arg1: *mut c_void)>;
pub type EMSCRIPTEN_WEBGL_CONTEXT_HANDLE = c_int;

#[repr(C)]
#[derive(Debug, Copy)]
pub struct EmscriptenWebGLContextAttributes {
    pub alpha: c_int,
    pub depth: c_int,
    pub stencil: c_int,
    pub antialias: c_int,
    pub premultipliedAlpha: c_int,
    pub preserveDrawingBuffer: c_int,
    pub preferLowPowerToHighPerformance: c_int,
    pub failIfMajorPerformanceCaveat: c_int,
    pub majorVersion: c_int,
    pub minorVersion: c_int,
    pub enableExtensionsByDefault: c_int,
    pub explicitSwapControl: c_int,
}

impl Clone for EmscriptenWebGLContextAttributes {
    fn clone(&self) -> Self {
        *self
    }
}

extern "C" {
    pub fn emscripten_set_main_loop_arg(
        func: em_arg_callback_func,
        arg: *mut c_void,
        fps: c_int,
        simulate_infinite_loop: c_int,
    );

    pub fn emscripten_GetProcAddress(name: *const c_char) -> *const c_void;

    pub fn emscripten_webgl_init_context_attributes(
        attributes: *mut EmscriptenWebGLContextAttributes,
    );

    pub fn emscripten_webgl_create_context(
        target: *const c_char,
        attributes: *const EmscriptenWebGLContextAttributes,
    ) -> EMSCRIPTEN_WEBGL_CONTEXT_HANDLE;

    pub fn emscripten_webgl_make_context_current(context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE)
        -> c_int;

    pub fn emscripten_get_element_css_size(
        target: *const c_char,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int;

    pub fn emscripten_asm_const_int(code: *const c_char, ...) -> c_int;
}
