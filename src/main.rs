extern crate gleam;
extern crate rand;

mod emscripten;
mod error;
mod matrix;
mod obj;
mod render;

use std::f32::consts::PI;
use std::fs::File;
use std::mem::size_of;

use emscripten::{
    emscripten_GetProcAddress, emscripten_get_element_css_size, emscripten_set_main_loop_arg,
    emscripten_webgl_create_context, emscripten_webgl_init_context_attributes,
    emscripten_webgl_make_context_current, EmscriptenWebGLContextAttributes,
};

use gleam::gl;
use gleam::gl::{GLenum, GLuint};

use matrix::{
    identity, matmul, orthogonal_matrix, perspective_matrix, rotate_x, rotate_y, scale, translate,
    viewing_matrix, Matrix44,
};
use obj::{vec3, Vec3};
use render::{Color, Desk, Drawable, Obj};

// Used for buffering data properly
const FLOAT_SIZE: usize = size_of::<f32>();
const U16_SIZE: usize = size_of::<u16>();
const U32_SIZE: usize = size_of::<u32>();

type GlPtr = std::rc::Rc<gl::Gl>;

#[repr(C)]
pub struct Context {
    gl: GlPtr,
    program: GLuint,
    buffer: Option<GLuint>,
    theta: f32,
    camera: Matrix44,
    p_matrix: Matrix44,
    width: u32,
    height: u32,
    objects: Vec<Box<Drawable>>,
}

fn load_shader(gl: &GlPtr, shader_type: GLenum, source: &[&[u8]]) -> Option<GLuint> {
    // Initialize an empty shader
    let shader = gl.create_shader(shader_type);
    // If initialization fails, return error
    if shader == 0 {
        return None;
    }
    // Load source into shader
    gl.shader_source(shader, source);
    // Compile shader
    gl.compile_shader(shader);
    // Check if shader compiled correctly
    let mut compiled = [0];
    unsafe {
        gl.get_shader_iv(shader, gl::COMPILE_STATUS, &mut compiled);
    }
    if compiled[0] == 0 {
        // Get shader compilation errors
        let log = gl.get_shader_info_log(shader);
        // Print to console
        println!("{}", log);
        // Delete shader
        gl.delete_shader(shader);
        // Return error
        None
    } else {
        // Return shader
        Some(shader)
    }
}

impl Context {
    fn init_buffer(&mut self) {
        let gl = &self.gl;
        // Parse the model
        let mut cat = Obj::load("/cat.obj", vec3(0.0, 0.5, 0.0)).unwrap();
        cat.set_group_color("".into(), Color::from_hex("a0522d").unwrap())
            .unwrap();
        // Set head col;ors
        /*head.set_group_color("Head".into(), Color::from_hex("ffe0bd").unwrap())
            .unwrap();
        head.set_group_color(
            "Eyes_Left_Eye_Ball".into(),
            Color::from_hex("a1caf1").unwrap(),
        )
        .unwrap();
        head.set_group_color(
            "Eyes_Right_Eye_Ball".into(),
            Color::from_hex("a1caf1").unwrap(),
        )
        .unwrap();*/
        // Load data from the head model
        let (mut vertices, mut elements) = cat.buffer_data(0, 0);
        // Add head to objects
        self.objects.push(Box::new(cat));
        // Create the table
        let mut table = Desk::new(4.0, 4.0, 0.2, 0.2, 0.2, 3.0);
        let (table_verts, table_elems) =
            table.buffer_data(elements.len() as GLuint, (vertices.len() / 6) as GLuint);

        vertices.extend_from_slice(&table_verts);
        elements.extend_from_slice(&table_elems);

        self.objects.push(Box::new(table));

        // Create gl data buffers
        let buffers = gl.gen_buffers(2);
        // Split into data and element buffers
        let vertex_buffer = buffers[0];
        let element_buffer = buffers[1];
        // Pull shader var locations from the shader program
        let position_location = gl.get_attrib_location(self.program, "aPosition") as u32;
        let color_location = gl.get_attrib_location(self.program, "aColor") as u32;
        // Set up arrays for loading buffers
        let array = gl.gen_vertex_arrays(1)[0];
        gl.bind_vertex_array(array);
        gl.enable_vertex_attrib_array(position_location);
        gl.enable_vertex_attrib_array(color_location);
        // Load vertex data into buffer
        gl.bind_buffer(gl::ARRAY_BUFFER, vertex_buffer);
        gl.buffer_data_untyped(
            gl::ARRAY_BUFFER,
            (FLOAT_SIZE as isize) * (vertices.len() as isize),
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );
        // Set offsets and load information for vertex data
        gl.vertex_attrib_pointer(
            position_location,
            3,
            gl::FLOAT,
            false,
            6 * FLOAT_SIZE as i32,
            0,
        );
        gl.vertex_attrib_pointer(
            color_location,
            3,
            gl::FLOAT,
            false,
            6 * FLOAT_SIZE as i32,
            3 * FLOAT_SIZE as u32,
        );
        // Load element data into buffer
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, element_buffer);
        gl.buffer_data_untyped(
            gl::ELEMENT_ARRAY_BUFFER,
            (U32_SIZE as isize) * (elements.len() as isize),
            elements.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );
        gl.bind_vertex_array(0);
        // Return vertex array pointer
        self.buffer = Some(array);
    }

    fn new(gl: GlPtr) -> Context {
        // Load and compile shaders
        let v_shader = load_shader(&gl, gl::VERTEX_SHADER, VS_SRC).unwrap();
        let f_shader = load_shader(&gl, gl::FRAGMENT_SHADER, FS_SRC).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, v_shader);
        gl.attach_shader(program, f_shader);
        gl.link_program(program);
        gl.use_program(program);
        // Get positions
        let position_location = gl.get_attrib_location(program, "aPosition") as u32;
        let color_location = gl.get_attrib_location(program, "aColor") as u32;
        // Configure position and color buffers for reading from arrays
        gl.enable_vertex_attrib_array(position_location);
        gl.enable_vertex_attrib_array(color_location);
        // Set gl to use a black background
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        // Enable depth testing
        gl.enable(gl::DEPTH_TEST);
        //gl.enable(gl::CULL_FACE);
        // Get canvas size
        let (width, height) = get_canvas_size();
        // Store all state
        Context {
            gl,
            program,
            buffer: None,
            theta: 0.0,
            // Set up view matrix
            camera: viewing_matrix(
                // eye
                [6.0, 3.0, 0.0],
                // up
                [0.0, 1.0, 0.0],
                //[0.0, 1.0, 0.0],
                // at
                [0.0, 0.0, 0.0],
            ),
            /*p_matrix: perspective_matrix(
                // FOV
                (60.0 as f32).to_radians(),
                // Aspect ratio
                width as f32 / height as f32,
                // Near plane
                0.01,
                // Far plane
                1000.0,
            ),*/
            #[cfg_attr(rustfmt, rustfmt_skip)]
            p_matrix: orthogonal_matrix(
                // Left, right
                -6.0, 6.0,
                // Top, bottom
                6.0, -6.0,
                // Near, far
                0.1, 1000.0
            ),
            width,
            height,
            objects: Vec::new(),
        }
    }

    fn draw(&self) {
        let gl = &self.gl;
        // Set view port
        gl.viewport(0, 0, self.width as i32, self.height as i32);
        // Clear the canvas
        gl.clear(gl::COLOR_BUFFER_BIT);
        // Set shader program
        gl.use_program(self.program);
        // Universally set perspective
        let p_location = gl.get_uniform_location(self.program, "uPMatrix");
        gl.uniform_matrix_4fv(p_location, false, &self.p_matrix);
        // Render each object
        gl.bind_vertex_array(self.buffer.unwrap());
        for object in &self.objects {
            object.draw(&self);
        }
        gl.bind_vertex_array(0);
    }
}

fn get_canvas_size() -> (u32, u32) {
    unsafe {
        let mut width = std::mem::uninitialized();
        let mut height = std::mem::uninitialized();
        emscripten_get_element_css_size(std::ptr::null(), &mut width, &mut height);
        (width as u32, height as u32)
    }
}

fn step(ctx: &mut Context) {
    ctx.theta -= 0.01;
    ctx.draw();
}

extern "C" fn loop_wrapper(ctx: *mut std::os::raw::c_void) {
    unsafe {
        let mut ctx = &mut *(ctx as *mut Context);
        step(&mut ctx);
    }
}

fn main() {
    unsafe {
        let mut attributes: EmscriptenWebGLContextAttributes = std::mem::uninitialized();
        emscripten_webgl_init_context_attributes(&mut attributes);
        attributes.majorVersion = 2;
        let handle = emscripten_webgl_create_context(std::ptr::null(), &attributes);
        emscripten_webgl_make_context_current(handle);
        let gl = gl::GlesFns::load_with(|addr| {
            let addr = std::ffi::CString::new(addr).unwrap();
            emscripten_GetProcAddress(addr.into_raw() as *const _) as *const _
        });
        let mut ctx = Context::new(gl);
        // Create a buffer for GL data
        ctx.init_buffer();
        let ptr = &mut ctx as *mut _ as *mut std::os::raw::c_void;
        emscripten_set_main_loop_arg(Some(loop_wrapper), ptr, 0, 1);
    }
}

const VS_SRC: &[&[u8]] = &[b"#version 300 es
layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aColor;
uniform mat4 uMVMatrix;
uniform mat4 uPMatrix;
out vec4 vColor;
void main() {
    gl_Position = uPMatrix * uMVMatrix * vec4(aPosition, 1.0);
    vColor = vec4(aColor, 1.0);
}"];

const FS_SRC: &[&[u8]] = &[b"#version 300 es
precision mediump float;
in vec4 vColor;
out vec4 oFragColor;
void main() {
    oFragColor = vColor;
}"];
