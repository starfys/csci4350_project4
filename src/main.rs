extern crate gleam;
extern crate image;

mod chair;
mod desk;
mod emscripten;
mod error;
mod extrusion;
mod matrix;
mod obj;
mod render;
mod revolution;
mod room;

pub extern "C" fn hello() {
    println!("hello");
}

use std::f32::consts::PI;
use std::mem::{self, size_of};
use std::ptr;

use emscripten::{
    emscripten_GetProcAddress, emscripten_asm_const_int, emscripten_get_element_css_size,
    emscripten_set_main_loop_arg, emscripten_webgl_create_context,
    emscripten_webgl_init_context_attributes, emscripten_webgl_make_context_current,
    EmscriptenWebGLContextAttributes,
};
use gleam::gl;
use gleam::gl::{GLenum, GLint, GLuint};

use chair::Chair;
use desk::Desk;
use matrix::{orthogonal_matrix, perspective_matrix, vec3, viewing_matrix, Matrix44, Vec3};
use obj::Obj;
use render::{star, Drawable};
use room::Room;

// Used for buffering data properly
const FLOAT_SIZE: usize = size_of::<f32>();

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
    animate: bool,
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

        // Keep track of texture indices
        let mut cur_texture: u8 = 0;

        // Create the room
        let room = Room::new(10.0, 10.0, 10.0);
        self.objects.push(Box::new(room));

        let clock = Obj::load(
            "/clock.obj",
            "grandfatherclock_uv.bmp",
            &mut cur_texture,
            // Half size
            vec3(1.0, 1.0, 1.0),
            // Behind the table
            vec3(0.0, 5.0, 0.0),
        ).unwrap();
        self.objects.push(Box::new(clock));


        let girl = Obj::load(
            "/girl.obj",
            "girl_texture.tga",
            &mut cur_texture,
            // Half size
            vec3(0.5, 0.5, 0.5),
            // Behind the table
            vec3(5.0, 4.0, 1.0),
        ).unwrap();
        self.objects.push(Box::new(girl));

        // Create the table
        let table = Desk::new(4.0, 4.0, 0.2, 0.2, 0.2, 3.0, vec3(5.0, 0.0, 5.0));
        self.objects.push(Box::new(table));

        let chair = Chair::new(1.0, 1.0, 0.2, 0.2, 0.2, 3.0, vec3(2.0, 0.0, 3.5));
        self.objects.push(Box::new(chair));

        let chair2 = Chair::new(1.0, 1.0, 0.2, 0.2, 0.2, 3.0, vec3(2.0, 0.0, 5.5));
        self.objects.push(Box::new(chair2));

        // Load the cat
        let cat = Obj::load(
            "/cat.obj",
            "/cat_diff.tga",
            &mut cur_texture,
            vec3(1.0, 1.0, 1.0),
            vec3(5.0, 3.5, 5.0),
        ).unwrap();
        self.objects.push(Box::new(cat));

        let star =
            extrusion::Extrusion::new(star(5, 0.3, 1.0), vec3(0.0, 0.5, 0.0), vec3(5.0, 8.0, 5.0));
        self.objects.push(Box::new(star));

        let staff = Obj::load(
            "/staff.obj",
            "/staff.tga",
            //"/cat_diff.tga",
            &mut cur_texture,
            vec3(1.0, 1.0, 1.0),
            vec3(7.0, 3.0, 7.0),
        ).unwrap();
        self.objects.push(Box::new(staff));

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let rot = revolution::Revolution::new(vec![
            vec3(0.5, 0.0, 0.0),
            vec3(0.55, 0.15, 0.0),
            vec3(0.5, 0.2, 0.0),
            vec3(0.4, 0.3, 0.0),
            vec3(0.15, 0.5, 0.0),
            vec3(0.15, 0.9, 0.0),
            vec3(0.175, 0.95, 0.0),
            vec3(0.15, 0.9, 0.0),
        ], 200, vec3(3.8, 3.3, 5.3));
        self.objects.push(Box::new(rot));

        //let mut potion = Obj::load("/potion.obj", vec3(5.0, 3.5, 5.0), 1).unwrap();
        //self.objects.push(Box::new(potion));

        // load texture data in here

        // Create a vertex buffer
        let mut vertices: Vec<f32> = Vec::new();
        // Buffer each object's data
        for mut object in &mut self.objects {
            let cur_verts = object.buffer_data(vertices.len() as GLint);
            vertices.extend_from_slice(&cur_verts);
        }
        // Load each object's textures
        for object in &self.objects {
            object.load_texture(&self);
        }

        // Parse the model
        //let mut potion = Obj::load("/potion.obj", vec3(5.5, 8.5, 5.5)).unwrap();
        // Load data from the cat model
        //let pot_verts = potion.buffer_data(vertices.len() as GLint);
        //vertices.extend_from_slice(&pot_verts);
        // Add head to objects
        //self.objects.push(Box::new(potion));

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
        gl.enable(gl::CULL_FACE);
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
                vec3(12.0, 12.0, 12.0),
                //vec3(5.0, 5.0, 10.0),
                //vec3(5.0, 10.0, 5.0),
                //vec3(0.0, 5.0, 0.0),
                //vec3(0.0, 10.0, 0.0),
                //vec3(0.0, 0.0, 10.0),

                // up
                //vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                // at
                vec3(0.0, 0.0, 0.0),
                //vec3(5.0, 0.0, 5.0),
            ),
            /*p_matrix: perspective_matrix(
                // FOV
                (60.0 as f32).to_radians(),
                // Aspect ratio
                width as f32 / height as f32,
                // Near plane
                0.1,
                // Far plane
                1000.0,
            ),*/
            #[cfg_attr(rustfmt, rustfmt_skip)]
            p_matrix: orthogonal_matrix(
                // Left, right
                -9.6, 9.6,
                // Top, bottom
                6.0, -6.0,
                // Near, far
                0.1, 1000.0
            ),
            width,
            height,
            objects: Vec::new(),
            animate: false,
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

        let light_position_location = gl.get_uniform_location(self.program, "uLightPosition");
        gl.uniform_3f(light_position_location, 5.0, 12.0, 5.0);

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
    // Check whether we should animate
    let code = "{return animate;}\0";
    let animate = unsafe { emscripten_asm_const_int(code.as_ptr() as *const _) };
    // Set animation state
    if animate == 0 && ctx.animate {
        ctx.animate = false;
    } else if animate == 1 && !ctx.animate {
        ctx.animate = true;
    }
    // Apply animation
    if ctx.animate {
        ctx.theta -= 0.1;
    }
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
uniform vec3 uLightPosition;
uniform float uShininess;

// Variables sent to fragment shader
out vec4 vColor;
out vec2 vTexCoord;

void main() {
    // Convert vertex and light position into camera coordinates
    vec3 pos = -(uMVMatrix * vec4(aPosition, 1.0)).xyz;
    // TODO: if this is uniform, why calculate it in each vertex
    vec3 light = -(uMVMatrix * vec4(uLightPosition, 1.0)).xyz;

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

    vTexCoord  = aTexture;
}

"
];

#[cfg_attr(rustfmt, rustfmt_skip)]
const FS_SRC: &[&[u8]] = &[
b"#version 300 es

precision mediump float;

in vec4 vColor;
in vec2 vTexCoord;

uniform sampler2D uSampler;

out vec4 oFragColor;

void main() {
    //oFragColor = vColor;
    oFragColor = vColor * texture(uSampler, vTexCoord);
}
"];
