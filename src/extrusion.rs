use super::Context;
use gleam::gl::{self, GLint, GLsizei};
use matrix::{identity, matmul, rotate_y, translate, vec3, Vec3};
use render::{polygon, quad, rectangular_prism, Color, Drawable, Vertex};

pub struct Extrusion {
    points: Vec<Vec3>,
    extrusion: Vec3,
    vert_start: GLint,
    num_verts: GLsizei,
    translate: Vec3,
}

impl Extrusion {
    pub fn new(points: Vec<Vec3>, extrusion: Vec3, translate: Vec3) -> Self {
        Extrusion {
            points,
            extrusion,
            vert_start: 0,
            num_verts: 0,
            translate,
        }
    }
}

impl Drawable for Extrusion {
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        self.vert_start = vertex_start;
        let mut vertices: Vec<Vertex> = polygon(&self.points);

        let top_verts: Vec<Vec3> = self
            .points
            .iter()
            .map(|vert| vert + self.extrusion)
            .collect();

        let sides: Vec<Vertex> = self
            .points
            .windows(2)
            .zip(top_verts.windows(2))
            .cycle()
            .take(self.points.len())
            .flat_map(|(b, t)| quad(t[0], b[0], b[1], t[1]).to_vec())
            .collect();

        vertices.extend_from_slice(&sides);

        vertices.extend_from_slice(&polygon(&top_verts));

        self.num_verts = vertices.len() as GLint;

        vertices
            .iter()
            .flat_map(|vertex| vertex.to_data().to_vec())
            .collect()
    }

    fn draw(&self, ctx: &Context) {
        let gl = &ctx.gl;
        let mv_location = gl.get_uniform_location(ctx.program, "uMVMatrix");
        let m_matrix = identity(); //translate(self.translate.x, self.translate.y, self.translate.z);
        let v_matrix = matmul(
            rotate_y(ctx.theta),
            matmul(
                translate(self.translate.x, self.translate.y, self.translate.z),
                ctx.camera,
            ),
        ); //matmul(rotate_y(ctx.theta), ctx.camera);
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

        gl.uniform_1f(shininess_location, 96.078_43);

        gl.draw_arrays(gl::TRIANGLES, self.vert_start / 8, self.num_verts);
    }
}
