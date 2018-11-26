extern crate gleam;
extern crate image;
extern crate rand;

mod emscripten;
mod error;
mod matrix;
mod obj;
mod render;

use std::f32::consts::PI;
use std::fs::File;
use std::mem::{self, size_of};
use std::ptr;

use emscripten::{
    emscripten_GetProcAddress, emscripten_get_element_css_size, emscripten_set_main_loop_arg,
    emscripten_webgl_create_context, emscripten_webgl_init_context_attributes,
    emscripten_webgl_make_context_current, EmscriptenWebGLContextAttributes,
};

use gleam::gl;
use gleam::gl::{GLenum, GLuint};
use image::{GenericImageView, Pixel};

use matrix::{
    identity, matmul, orthogonal_matrix, perspective_matrix, rotate_x, rotate_y, scale, translate,
    viewing_matrix, Matrix44,
};
use obj::{vec3, Obj, Vec3};
use render::{Color, Desk, Drawable};

// Used for buffering data properly
const FLOAT_SIZE: usize = size_of::<f32>();
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
        // Load the texture file
        let cat_texture = image::open("/cat_diff.tga").unwrap();
        // Extract dimensions
        let (width, height) = cat_texture.dimensions();
        // Get image as raw bytes
        let cat_texture = cat_texture.as_rgb8().unwrap().clone();
        let texture = gl.gen_textures(1)[0];

        // load texture data in here

        gl.active_texture(gl::TEXTURE0);
        gl.bind_texture(gl::TEXTURE_2D_ARRAY, texture);
        gl.tex_parameter_i(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );
        gl.tex_parameter_i(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );
        gl.tex_image_3d(
            gl::TEXTURE_2D_ARRAY,
            0,
            gl::RGB as i32,
            width as i32,
            height as i32,
            1,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            Some(&cat_texture),
        );
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
        let vertices = cat.buffer_data(0);
        // Add head to objects
        self.objects.push(Box::new(cat));
        // Create the table
        /*let mut table = Desk::new(4.0, 4.0, 0.2, 0.2, 0.2, 3.0);
        let (table_verts, table_elems) =
            table.buffer_data(elements.len() as GLuint, (vertices.len() / 6) as GLuint);

        vertices.extend_from_slice(&table_verts);
        elements.extend_from_slice(&table_elems);

        self.objects.push(Box::new(table));
        */
        // Create gl data buffers
        let buffers = gl.gen_buffers(2);
        // Split into data and element buffers
        let vertex_buffer = buffers[0];
        let _element_buffer = buffers[1];
        // Pull attribute locations from the shader program
        let position_location = gl.get_attrib_location(self.program, "aPosition") as u32;
        let normal_location = gl.get_attrib_location(self.program, "aNormal") as u32;
        let texture_location = gl.get_attrib_location(self.program, "aTexture") as u32;
        // Set up arrays for loading buffers
        let array = gl.gen_vertex_arrays(1)[0];
        gl.bind_vertex_array(array);
        gl.enable_vertex_attrib_array(position_location);
        gl.enable_vertex_attrib_array(normal_location);
        gl.enable_vertex_attrib_array(texture_location);

        // Load vertex data into buffer
        gl.bind_buffer(gl::ARRAY_BUFFER, vertex_buffer);
        gl.buffer_data_untyped(
            gl::ARRAY_BUFFER,
            (FLOAT_SIZE as isize) * (vertices.len() as isize),
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );
        // Set offsets and load information for vertex positions
        gl.vertex_attrib_pointer(
            position_location,
            3,
            gl::FLOAT,
            false,
            8 * FLOAT_SIZE as i32,
            0,
        );
        // Set offsets and load information for vertex normals
        gl.vertex_attrib_pointer(
            normal_location,
            3,
            gl::FLOAT,
            false,
            8 * FLOAT_SIZE as i32,
            3 * FLOAT_SIZE as u32,
        );
        // Set offsets and load information for vertex texture coordinates
        gl.vertex_attrib_pointer(
            texture_location,
            2,
            gl::FLOAT,
            false,
            8 * FLOAT_SIZE as i32,
            6 * FLOAT_SIZE as u32,
        );
        // ???
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
        let mut width = mem::uninitialized();
        let mut height = mem::uninitialized();
        emscripten_get_element_css_size(ptr::null(), &mut width, &mut height);
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
        let mut attributes: EmscriptenWebGLContextAttributes = mem::uninitialized();
        emscripten_webgl_init_context_attributes(&mut attributes);
        attributes.majorVersion = 2;
        let handle = emscripten_webgl_create_context(ptr::null(), &attributes);
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

#[cfg_attr(rustfmt, rustfmt_skip)]
const VS_SRC: &[&[u8]] = &[
b"#version 300 es

// Per-vertex attributes
layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec2 aTexture;

// All-vertex uniforms
// MV matrix
uniform mat4 uMVMatrix;
// Perspective matrix
uniform mat4 uPMatrix;
// Lighting properties
uniform vec4 uAmbientProduct;
uniform vec4 uDiffuseProduct;
uniform vec4 uSpecularProduct;
// Light position
uniform vec4 uLightPosition;
uniform float uShininess;

// Variable sent to fragment shader
out vec4 vColor;


void main() {
    // Convert vertex and light position into camera coordinates
    vec3 pos = -(uMVMatrix * vec4(aPosition, 1.0)).xyz;
    // TODO: if this is uniform, why calculate it in each vertex
    vec3 light = -(uMVMatrix * uLightPosition).xyz;

    // light source direction
    vec3 L = normalize(light - pos);
    
    // eye - point location  (eye is at origin of eye frame)
    vec3 E = normalize(-pos); 
    
    // Half-way vector
    vec3 H = normalize(L + E);

    // Transform vertex normal into eye coordinates
    vec3 N = normalize((uMVMatrix * vec4(aNormal, 1.0)).xyz);

    // Compute terms in the illumination equation
    
    // ambient is already given
    
    float Kd = max(dot(L, N), 0.0);
    vec4 diffuse = Kd * uDiffuseProduct;

    float Ks = pow(max(dot(N, H), 0.0), uShininess);
    vec4 specular = Ks * uSpecularProduct;
    
    if( dot(L, N) < 0.0 )  specular = vec4(0.0, 0.0, 0.0, 1.0);

    gl_Position = uPMatrix * uMVMatrix * vec4(aPosition, 1.0);
    
    vColor = uAmbientProduct + diffuse + specular;

    vColor.a = 1.0;
}

"
];

#[cfg_attr(rustfmt, rustfmt_skip)]
const FS_SRC: &[&[u8]] = &[
b"#version 300 es

precision mediump float;


in vec4 vColor;

out vec4 oFragColor;

void main() {
    oFragColor = vColor;
}
"];
