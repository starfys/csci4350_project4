use super::Context;
use gleam::gl::{self, GLint, GLsizei};
use matrix::{identity, matmul, rotate_y, translate, vec3, Vec3};
use render::{rectangular_prism, Color, Drawable, Vertex};

pub struct Chair {
    top_width: f32,
    top_height: f32,
    top_depth: f32,
    leg_width: f32,
    leg_height: f32,
    leg_depth: f32,
    vert_start: GLint,
    num_verts: GLsizei,
    translate: Vec3,
}

impl Chair {
    pub fn new(
        top_width: f32,
        top_height: f32,
        top_depth: f32,
        leg_width: f32,
        leg_height: f32,
        leg_depth: f32,
        translate: Vec3,
    ) -> Self {
        Chair {
            top_width,
            top_height,
            top_depth,
            leg_width,
            leg_height,
            leg_depth,
            vert_start: 0,
            num_verts: 0,
            translate,
        }
    }
}
impl Drawable for Chair {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        // Store the vertex starting pointer
        self.vert_start = vertex_start;
        // Create buffers for vertices and elements
        let mut vertices: Vec<Vertex> = Vec::new();
        // Generate vertices for table top
        let top_vertices = rectangular_prism(
            &vec3(0.0, (self.leg_depth + self.top_depth / 2.0) - self.leg_depth / 4.0, 0.0) + self.translate,
            self.top_width,
            self.top_height,
            self.top_depth,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&top_vertices);
        // Generate vertices for legs
        // near left leg
        let nl_leg_vertices = rectangular_prism(
            &vec3(
                -self.top_width / 2.0 + self.leg_width / 2.0,
                self.leg_depth / 2.0,
                -self.top_height / 2.0 + self.leg_height / 2.0,
            ) + self.translate,
            self.leg_width,
            self.leg_height,
            self.leg_depth / 2.0,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&nl_leg_vertices);
        // near right leg
        let nr_leg_vertices = rectangular_prism(
            &vec3(
                self.top_width / 2.0 - self.leg_width / 2.0,
                self.leg_depth / 2.0,
                -self.top_height / 2.0 + self.leg_height / 2.0,
            ) + self.translate,
            self.leg_width,
            self.leg_height,
            self.leg_depth / 2.0,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&nr_leg_vertices);
        // far left leg
        let fl_leg_vertices = rectangular_prism(
            &vec3(
                -self.top_width / 2.0 + self.leg_width / 2.0,
                self.leg_depth / 2.0,
                self.top_height / 2.0 - self.leg_height / 2.0,
            ) + self.translate,
            self.leg_width,
            self.leg_height,
            self.leg_depth / 2.0,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&fl_leg_vertices);
        // far right leg
        let fr_leg_vertices = rectangular_prism(
            &vec3(
                self.top_width / 2.0 - self.leg_width / 2.0,
                self.leg_depth / 2.0,
                self.top_height / 2.0 - self.leg_height / 2.0,
            ) + self.translate,
            self.leg_width,
            self.leg_height,
            self.leg_depth / 2.0,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&fr_leg_vertices);
        // Back of chair
        /*let back_vertices = rectangular_prism(
            &vec3(
                -self.top_width / 2.0 + self.leg_width / 2.0,
                (self.leg_depth / 2.0) + 2.0,
                self.top_height / 2.0,
            ) + self.translate,
            self.top_width,
            self.leg_height / 2.0,
            self.leg_depth / 2.0,
        );

        vertices.extend_from_slice(&back_vertices);
        */

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
        let m_matrix = identity(); //translate(self.translate.x, self.translate.y, self.translate.z);
        let v_matrix = ctx.camera; //matmul(rotate_y(ctx.theta), ctx.camera);
        let mv_matrix = matmul(v_matrix, m_matrix);
        gl.uniform_matrix_4fv(mv_location, false, &mv_matrix);

        // Lighting properties
        let ambient_location = gl.get_uniform_location(ctx.program, "uAmbientProduct");
        let diffuse_location = gl.get_uniform_location(ctx.program, "uDiffuseProduct");
        let specular_location = gl.get_uniform_location(ctx.program, "uSpecularProduct");
        // Light position
        let shininess_location = gl.get_uniform_location(ctx.program, "uShininess");

        // Set lighting properties
        gl.uniform_4f(ambient_location, 0.396, 0.263, 0.129, 1.0);
        gl.uniform_4f(diffuse_location, 0.64, 0.64, 0.64, 1.0);
        gl.uniform_4f(specular_location, 0.0, 0.0, 0.0, 1.0);

        gl.uniform_1f(shininess_location, 96.078431);

        gl.draw_arrays(gl::TRIANGLES, self.vert_start / 8, self.num_verts);
    }
}
