use std::collections::HashMap;
use std::f32::consts::PI;
use std::io;
use std::path::Path;

use gleam::gl;
use gleam::gl::types::{GLsizei, GLuint};
use matrix::{identity, matmul, rotate_x, rotate_y, scale, translate};
use rand::Rng;

use super::{Context, U32_SIZE};
use error::io_error;
use obj::{load_obj, vec3, Group, Vec3};

pub trait Drawable {
    /// Returns buffer data
    fn buffer_data(&mut self, elem_start: GLuint, vertex_start: GLuint) -> (Vec<f32>, Vec<u32>);
    /// Draws the shape
    fn draw(&self, gl: &Context);
}
pub struct Obj {
    groups: Vec<Group>,
    colors: HashMap<String, Color>,
    elem_start: GLuint,
    num_elems: GLsizei,
    vert_start: GLuint,
    num_verts: GLsizei,
    center: Vec3,
    translate: Vec3,
}

impl Obj {
    /// Loads a render object from a path
    pub fn load<P: AsRef<Path>>(path: P, translate: Vec3) -> Result<Self, io::Error> {
        // Parse object file
        let groups = load_obj(path)?;
        // Generate the render object
        Ok(Obj {
            groups,
            colors: HashMap::new(),
            elem_start: 0,
            num_elems: 0,
            vert_start: 0,
            num_verts: 0,
            center: Vec3::origin(),
            translate,
        })
    }
    /// Sets the color for a group
    pub fn set_group_color(&mut self, group_name: String, color: Color) -> Result<(), io::Error> {
        // Check to see if the group name corresponds to a loaded group
        if self
            .groups
            .iter()
            .find(|ref group| group.name == group_name)
            .is_some()
        {
            self.colors.insert(group_name, color);
            Ok(())
        } else {
            Err(io_error("Invalid group name"))
        }
    }
}
impl Drawable for Obj {
    /// Returns buffer data
    fn buffer_data(&mut self, elem_start: GLuint, vertex_start: GLuint) -> (Vec<f32>, Vec<u32>) {
        // Store element start
        self.elem_start = elem_start;
        self.vert_start = vertex_start;
        // Store vertex data
        let mut vertices: Vec<f32> = Vec::new();
        // Store index data
        let mut indices: Vec<u32> = Vec::new();
        // Iterate over groups
        for group in &self.groups {
            let def = Color::default();
            // Get the group's color. If none is provided, use the default (white)
            let color = self.colors.get(&group.name).unwrap_or(&def);
            // Extract data for the current group
            let (cur_vertices, cur_indices, center, size) = group.to_draw_info(&color);
            self.center = center;
            // Modify element indices to acknowledge offset
            let cur_indices = cur_indices.iter().map(|index| index + (elem_start as u32));
            // Modify indices to start further in the array
            // Add existing data
            vertices.extend_from_slice(&cur_vertices);
            indices.extend(cur_indices);
        }
        // Store element end
        self.num_elems = indices.len() as GLsizei;
        self.num_verts = vertices.len() as GLsizei;
        (vertices, indices)
    }
    /// Draws the object
    fn draw(&self, ctx: &Context) {
        let gl = &ctx.gl;
        let mv_location = gl.get_uniform_location(ctx.program, "uMVMatrix");
        /*let Vec3 {
            x: c_x,
            y: c_y,
            z: c_z,
        } = self.center;
        let m_matrix = translate(-c_x, -c_y, -c_z);*/
        let m_matrix = identity();
        let Vec3 { x, y, z } = self.translate;
        let m_matrix = matmul(translate(x, y, z), m_matrix);
        let v_matrix = matmul(rotate_y(ctx.theta), ctx.camera);
        let mv_matrix = matmul(v_matrix, m_matrix);
        gl.uniform_matrix_4fv(mv_location, false, &mv_matrix);
        gl.draw_elements(
            gl::TRIANGLES,
            self.num_elems,
            gl::UNSIGNED_INT,
            self.elem_start * (U32_SIZE as u32),
        );
    }
}

#[derive(Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Default for Color {
    fn default() -> Color {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

fn hex_to_byte(hex: &str) -> Result<u8, io::Error> {
    let mut result: u8 = 0;
    for h in hex.chars() {
        let h = match h {
            '0' => 0x0,
            '1' => 0x1,
            '2' => 0x2,
            '3' => 0x3,
            '4' => 0x4,
            '5' => 0x5,
            '6' => 0x6,
            '7' => 0x7,
            '8' => 0x8,
            '9' => 0x9,
            'a' | 'A' => 0xa,
            'b' | 'B' => 0xb,
            'c' | 'C' => 0xc,
            'd' | 'D' => 0xd,
            'e' | 'E' => 0xe,
            'f' | 'F' => 0xf,
            _ => return Err(io_error("Invalid char")),
        };
        result <<= 4;
        result += h;
    }
    Ok(result)
}
impl Color {
    pub fn from_hex(hex: &str) -> Result<Color, io::Error> {
        // Remove first character if it is '#'
        // TODO: better way to do this
        let (_, hex) = if hex.chars().next() == Some('#') {
            hex.split_at(1)
        } else {
            ("", hex)
        };
        // Split off red
        let (r, hex) = hex.split_at(2);
        let r: f32 = f32::from(hex_to_byte(r)?) / 255.0;
        // Split off green
        let (g, hex) = hex.split_at(2);
        let g: f32 = f32::from(hex_to_byte(g)?) / 255.0;
        // Split off blue
        let (b, hex) = hex.split_at(2);
        let b: f32 = f32::from(hex_to_byte(b)?) / 255.0;
        // Check if there are remaining characters, and find alpha based on result
        let a: f32 = match hex.chars().count() {
            // If no alpha is given, default to 1.0
            0 => 1.0,
            // If 1 character is given, there is insufficient data
            1 => {
                return Err(io_error("Insufficient data to calculate alpha"));
            }
            // If alpha is given, parse it
            2 => {
                // If there are only 2 remaining characters, just parse them
                f32::from(hex_to_byte(hex)?) / 255.0
            }
            // If there are trailing characters
            _ => {
                return Err(io_error("Trailing characters found"));
            }
        };
        // Return final result
        Ok(Color { r, g, b, a })
    }
    fn random() -> Color {
        let mut rng = rand::thread_rng();
        Color {
            r: rng.gen_range(0.0, 1.0),
            g: rng.gen_range(0.0, 1.0),
            b: rng.gen_range(0.0, 1.0),
            a: 1.0,
        }
    }
}

#[cfg(test)]
mod test {
    use std::io;

    use super::Color;

    #[test]
    fn test_color() -> io::Result<()> {
        assert_eq!(
            Color::from_hex("00ff00")?,
            Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0
            }
        );
        assert_eq!(
            Color::from_hex("#00ff00")?,
            Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0
            }
        );
        assert_eq!(
            Color::from_hex("#00ff0000")?,
            Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 0.0,
            }
        );
        Ok(())
    }
}

pub struct Hat {
    brim_inner_radius: f32,
    brim_outer_radius: f32,
    cone_bottom_radius: f32,
    cone_height: f32,
    subdivisions: usize,
}

impl Hat {
    fn new(
        brim_inner_radius: f32,
        brim_outer_radius: f32,
        cone_bottom_radius: f32,
        cone_height: f32,
        subdivisions: usize,
    ) -> Self {
        Hat {
            brim_inner_radius,
            brim_outer_radius,
            cone_bottom_radius,
            cone_height,
            subdivisions,
        }
    }
    /// Returns buffer data
    pub fn buffer_data(&self, vertex_start: u32) -> (Vec<f32>, Vec<u32>) {
        // Create buffers for vertices and elements
        let vertices: Vec<f32> = Vec::new();
        let buffer: Vec<u32> = Vec::new();
        // Add points for
        (vec![], vec![])
    }
}

pub struct Desk {
    top_width: f32,
    top_height: f32,
    top_depth: f32,
    leg_width: f32,
    leg_height: f32,
    leg_depth: f32,
    pub elem_start: GLuint,
    pub num_elems: GLsizei,
    vert_start: GLuint,
}

impl Desk {
    pub fn new(
        top_width: f32,
        top_height: f32,
        top_depth: f32,
        leg_width: f32,
        leg_height: f32,
        leg_depth: f32,
    ) -> Self {
        Desk {
            top_width,
            top_height,
            top_depth,
            leg_width,
            leg_height,
            leg_depth,
            elem_start: 0,
            num_elems: 0,
            vert_start: 0,
        }
    }
}
impl Drawable for Desk {
    /// Returns buffer data
    fn buffer_data(&mut self, elem_start: GLuint, vertex_start: GLuint) -> (Vec<f32>, Vec<u32>) {
        // Create buffers for vertices and elements
        let mut vertices: Vec<Vec3> = Vec::new();
        let mut elements: Vec<u32> = Vec::new();
        self.elem_start = elem_start;
        self.vert_start = vertex_start;
        // Start keeping track of index
        let mut cur_index = vertex_start;
        // Generate vertices for table top
        let (top_vertices, top_indices) = rectangular_prism(
            vec3(0.0, self.leg_depth + self.top_depth / 2.0, 0.0),
            self.top_width,
            self.top_height,
            self.top_depth,
            cur_index,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&top_vertices);
        elements.extend_from_slice(&top_indices);
        // Update index
        cur_index = vertex_start + (vertices.len() as GLuint);
        // Generate vertices for legs
        // near left leg
        let (nl_leg_vertices, nl_leg_indices) = rectangular_prism(
            vec3(
                -self.top_width / 2.0 + self.leg_width / 2.0,
                self.leg_depth / 2.0,
                -self.top_height / 2.0 + self.leg_height / 2.0,
            ),
            //vec3(0.0, 0.0, 0.0),
            self.leg_width,
            self.leg_height,
            self.leg_depth,
            cur_index,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&nl_leg_vertices);
        elements.extend_from_slice(&nl_leg_indices);
        // Update index
        cur_index = vertex_start + (vertices.len() as GLuint);
        // near right leg
        let (nr_leg_vertices, nr_leg_indices) = rectangular_prism(
            vec3(
                self.top_width / 2.0 - self.leg_width / 2.0,
                self.leg_depth / 2.0,
                -self.top_height / 2.0 + self.leg_height / 2.0,
            ),
            //vec3(0.0, 0.0, 0.0),
            self.leg_width,
            self.leg_height,
            self.leg_depth,
            cur_index,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&nr_leg_vertices);
        elements.extend_from_slice(&nr_leg_indices);
        // Update index
        cur_index = vertex_start + (vertices.len() as GLuint);
        // far left leg
        let (fl_leg_vertices, fl_leg_indices) = rectangular_prism(
            vec3(
                -self.top_width / 2.0 + self.leg_width / 2.0,
                self.leg_depth / 2.0,
                self.top_height / 2.0 - self.leg_height / 2.0,
            ),
            //vec3(0.0, 0.0, 0.0),
            self.leg_width,
            self.leg_height,
            self.leg_depth,
            cur_index,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&fl_leg_vertices);
        elements.extend_from_slice(&fl_leg_indices);
        // Update index
        cur_index = vertex_start + (vertices.len() as GLuint);
        // far right leg
        let (fr_leg_vertices, fr_leg_indices) = rectangular_prism(
            vec3(
                self.top_width / 2.0 - self.leg_width / 2.0,
                self.leg_depth / 2.0,
                self.top_height / 2.0 - self.leg_height / 2.0,
            ),
            //vec3(0.0, 0.0, 0.0),
            self.leg_width,
            self.leg_height,
            self.leg_depth,
            cur_index,
        );
        // Add vertices and indices
        vertices.extend_from_slice(&fr_leg_vertices);
        elements.extend_from_slice(&fr_leg_indices);
        // Update index
        cur_index = vertex_start + (vertices.len() as GLuint);

        // Store element end
        self.num_elems = elements.len() as GLsizei;

        // Add colors
        let colors = vertices.iter().map(|_| Color::from_hex("#808080").unwrap());
        // Flatten vertices and add colors
        let vertices = vertices
            .iter()
            .zip(colors)
            .map(|(vertex, color)| vec![vertex.x, vertex.y, vertex.z, color.r, color.g, color.b])
            .flatten()
            .collect();
        // Add points for
        (vertices, elements)
    }
    /// Draws the object
    fn draw(&self, ctx: &Context) {
        let gl = &ctx.gl;
        let mv_location = gl.get_uniform_location(ctx.program, "uMVMatrix");
        let m_matrix = translate(0.0, -3.0, 0.0);
        let v_matrix = matmul(rotate_y(ctx.theta), ctx.camera);
        let mv_matrix = matmul(v_matrix, m_matrix);
        gl.uniform_matrix_4fv(mv_location, false, &mv_matrix);
        gl.draw_elements(
            gl::TRIANGLES,
            self.num_elems,
            gl::UNSIGNED_INT,
            self.elem_start * (U32_SIZE as u32),
        );
    }
}

// Helper functions
/// Converts quad to tris
fn quad<T>(a: T, b: T, c: T, d: T) -> [T; 6]
where
    T: Copy,
{
    [a, b, c, c, d, a]
}
/// Generates a rectangular_prism, cen
fn rectangular_prism(
    center: Vec3,
    width: f32,
    height: f32,
    depth: f32,
    vertex_start: GLuint,
) -> (Vec<Vec3>, Vec<u32>) {
    // Easy access to self elements
    // Start by creating the table top
    // FRONT
    // (view from front)
    // 0--3
    // |  |
    // 1--2
    // 0
    let front_top_left = vec3(-width / 2.0, depth / 2.0, -height / 2.0);
    // 1
    let front_bottom_left = vec3(-width / 2.0, -depth / 2.0, -height / 2.0);
    // 2
    let front_bottom_right = vec3(width / 2.0, -depth / 2.0, -height / 2.0);
    // 3
    let front_top_right = vec3(width / 2.0, depth / 2.0, -height / 2.0);
    // BACK
    // (view from front)
    // 4--7
    // |  |
    // 5--6
    // 4
    let back_top_left = vec3(-width / 2.0, depth / 2.0, height / 2.0);
    // 5
    let back_bottom_left = vec3(-width / 2.0, -depth / 2.0, height / 2.0);
    // 6
    let back_bottom_right = vec3(width / 2.0, -depth / 2.0, height / 2.0);
    // 7
    let back_top_right = vec3(width / 2.0, depth / 2.0, height / 2.0);
    // Add all vertices to vertices array
    let vertices = vec![
        front_top_left,
        front_bottom_left,
        front_bottom_right,
        front_top_right,
        back_top_left,
        back_bottom_left,
        back_bottom_right,
        back_top_right,
    ]
    .iter()
    .map(|vert| vert + center)
    .collect();
    // Create buffer for elements
    let mut elements: Vec<u32> = Vec::new();
    // Add proper indices
    // Front
    elements.extend_from_slice(&quad(0, 1, 2, 3));
    // Back
    elements.extend_from_slice(&quad(7, 6, 5, 4));
    // Left
    elements.extend_from_slice(&quad(4, 5, 1, 0));
    // Right

    elements.extend_from_slice(&quad(3, 2, 6, 7));
    // Top
    elements.extend_from_slice(&quad(4, 0, 3, 7));

    // Bottom
    elements.extend_from_slice(&quad(1, 5, 6, 2));
    // Add necessary value to elements

    let elements = elements
        .iter()
        .map(|elem| elem + (vertex_start as u32))
        .collect();

    (vertices, elements)
}
