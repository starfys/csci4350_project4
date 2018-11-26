use super::Context;
use gleam::gl::{self, GLint, GLsizei};
use matrix::{identity, matmul, rotate_y, translate, vec3, Vec3};
use render::{quad, rectangular_prism, Color, Drawable, Vertex};

pub struct Revolution {
    path: Vec<Vec3>,
    vert_start: GLint,
    num_verts: GLsizei,
}

impl Revolution {
    pub fn new(path: Vec<Vec3>) -> Revolution {
        Revolution {
            path,
            vert_start: 0,
            num_verts: 0,
        }
    }
}
impl Drawable for Revolution {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        // Store the vertex starting pointer
        self.vert_start = vertex_start;
        // Start making vertices
        let vertices: Vec<Vertex> = Vec::new();
        // Vertices
        self.num_verts = vertices.len() as GLint;

        // Flatten vertices and add colors
        let vertices = vertices
            .iter()
            .flat_map(|vertex| vertex.to_data().to_vec())
            .collect();
        vertices
    }
    /// Draws the object
    fn draw(&self, ctx: &Context) {
        let gl = &ctx.gl;
        let mv_location = gl.get_uniform_location(ctx.program, "uMVMatrix");
        let m_matrix = identity();
        let v_matrix = matmul(rotate_y(ctx.theta), ctx.camera);
        let mv_matrix = matmul(v_matrix, m_matrix);
        gl.uniform_matrix_4fv(mv_location, false, &mv_matrix);

        // Lighting properties
        let ambient_location = gl.get_uniform_location(ctx.program, "uAmbientProduct");
        let diffuse_location = gl.get_uniform_location(ctx.program, "uDiffuseProduct");
        let specular_location = gl.get_uniform_location(ctx.program, "uSpecularProduct");
        // Light position
        let light_position_location = gl.get_uniform_location(ctx.program, "uLightPosition");
        let shininess_location = gl.get_uniform_location(ctx.program, "uShininess");

        // Set lighting properties
        gl.uniform_4f(ambient_location, 0.6, 0.6, 0.6, 1.0);
        gl.uniform_4f(diffuse_location, 0.64, 0.64, 0.64, 1.0);
        gl.uniform_4f(specular_location, 0.0, 0.0, 0.0, 1.0);

        gl.uniform_1f(shininess_location, 40.078431);

        gl.draw_arrays(gl::TRIANGLES, self.vert_start / 8, self.num_verts);
    }
}
