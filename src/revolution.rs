use super::Context;
use gleam::gl::{self, GLint, GLsizei};
use matrix::{identity, matmul, rotate_y, translate, vec3, Vec3};
use render::{quad, rectangular_prism, Color, Drawable, Vertex};

pub struct Revolution {
    vert_start: GLint,
    num_verts: GLsizei,
}

impl Revolution {
    pub fn new(path: Vec<Vec3>) {
        Room {
            room_width,
            room_height,
            room_depth,
            vert_start: 0,
            num_verts: 0,
        }
    }
}
impl Drawable for Room {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        // Store the vertex starting pointer
        self.vert_start = vertex_start;
        // Layout of the room
        //        y
        // LTL----MT----RTR
        // |      ||      |
        // |h     ||      |
        // |   w  ||   d  |
        //zLBR----MB----RBR
        // \      00      /
        //  \            /
        //   \ d       w/
        //    \        /
        //     \      /
        //      \    /
        //       \  /
        //        \/
        //        MF
        //        x
        // Create points
        let ltl = vec3(0.0, self.room_height, self.room_width);
        let lbr = vec3(0.0, 0.0, self.room_width);
        let mb = vec3(0.0, 0.0, 0.0);
        let mt = vec3(0.0, self.room_height, 0.0);
        let rbr = vec3(self.room_depth, 0.0, 0.0);
        let rtr = vec3(self.room_depth, self.room_height, 0.0);
        let mf = vec3(self.room_depth, 0.0, self.room_width);
        // Create vertex buffer
        let mut vertices: Vec<Vertex> = Vec::with_capacity(18);
        vertices.extend_from_slice(&quad(ltl, lbr, mb, mt));
        vertices.extend_from_slice(&quad(mt, mb, rbr, rtr));
        vertices.extend_from_slice(&quad(mb, lbr, mf, rbr));

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
