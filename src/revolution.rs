use std::f32::consts::PI;

use gleam::gl::{self, GLint, GLsizei};

use super::Context;
use matrix::{identity, matmul, rotate_y, translate, vec3, Vec3};
use render::{quad, rectangular_prism, tri, Color, Drawable, Vertex};

/// Takes a path and rotates it about the Y axis
pub struct Revolution {
    path: Vec<Vec3>,
    resolution: u16,
    vert_start: GLint,
    num_verts: GLsizei,
    translate: Vec3,
}

impl Revolution {
    pub fn new(path: Vec<Vec3>, resolution: u16, translate: Vec3) -> Revolution {
        Revolution {
            path,
            resolution,
            vert_start: 0,
            num_verts: 0,
            translate,
        }
    }
}
impl Drawable for Revolution {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        // Store the vertex starting pointer
        self.vert_start = vertex_start;
        // Start making vertices
        let mut vertices: Vec<Vertex> = Vec::new();
        // Start with the path
        let mut path: Vec<Vec3> = self.path.clone();

        // Get revolution amount per step
        let theta = (2.0 * PI) / f32::from(self.resolution);
        // Apply revolutions
        for _ in 0..self.resolution {
            // Rotate the path about the y axis some split amount
            let rotated_path: Vec<Vec3> = path.iter().map(|v| v.rotate_y(theta)).collect();
            // First (top/bottom) triangle
            vertices.extend_from_slice(&tri(path[0], rotated_path[0], vec3(0.0, path[0].y, 0.0)));

            // Make quads to connect rotated paths
            for pair in path.windows(2).zip(rotated_path.windows(2)) {
                // Match on guaranteed window pattern
                if let (&[a, b], &[c, d]) = pair {
                    vertices.extend_from_slice(&quad(a, c, d, b))
                };
            }

            // Last (top/bottom) triangle
            vertices.extend_from_slice(&tri(
                path[path.len() - 1],
                rotated_path[path.len() - 1],
                vec3(0.0, path[path.len() - 1].y, 0.0),
            ));

            path = rotated_path;
        }
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
        let m_matrix = translate(self.translate.x, self.translate.y, self.translate.z);
        let v_matrix = ctx.camera;
        let mv_matrix = matmul(v_matrix, m_matrix);
        gl.uniform_matrix_4fv(mv_location, false, &mv_matrix);

        // Lighting properties
        let ambient_location = gl.get_uniform_location(ctx.program, "uAmbientProduct");
        let diffuse_location = gl.get_uniform_location(ctx.program, "uDiffuseProduct");
        let specular_location = gl.get_uniform_location(ctx.program, "uSpecularProduct");
        // Light position
        let shininess_location = gl.get_uniform_location(ctx.program, "uShininess");

        // Set lighting properties
        //gl.uniform_4f(ambient_location, 0.6, 0.6, 0.6, 1.0);
        gl.uniform_4f(ambient_location, 0.6, 0.0, 0.0, 1.0);
        gl.uniform_4f(diffuse_location, 0.64, 0.64, 0.64, 1.0);
        gl.uniform_4f(specular_location, 0.0, 0.0, 0.0, 1.0);
        gl.uniform_1f(shininess_location, 40.078431);

        gl.draw_arrays(gl::TRIANGLES, self.vert_start / 8, self.num_verts);
    }
}
