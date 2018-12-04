use std::collections::HashMap;
use std::f32::consts::PI;
use std::io;
use std::path::Path;

use gleam::gl;
use gleam::gl::types::{GLenum, GLint, GLsizei};

use super::Context;
use error::io_error;
use matrix::{identity, matmul, rotate_x, rotate_y, scale, translate, vec3, Vec2, Vec3};

pub trait Drawable {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32>;
    /// Loads texture data
    fn load_texture(&self, ctx: &Context) {}
    /// Draws the shape
    fn draw(&self, ctx: &Context);
}

/// Used to represent data buffered into vertex
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    texture: Vec2,
}
/// Creates a vertex without texture coords
pub fn vertex(position: Vec3, normal: Vec3) -> Vertex {
    Vertex {
        position,
        normal,
        texture: Vec2::origin(),
    }
}
impl Vertex {
    pub fn to_data(&self) -> [f32; 8] {
        [
            self.position.x,
            self.position.y,
            self.position.z,
            self.normal.x,
            self.normal.y,
            self.normal.z,
            self.texture.x,
            self.texture.y,
        ]
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
        let (_, hex) = if hex.starts_with('#') {
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

// Helper functions
// Newell Method for surface normal calculation
pub fn newell(points: Vec<Vec3>) -> Vec3 {
    /*let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    for (index, element) in points.iter().enumerate() {
        let current_element = element;
        let next_element = points[(index + 1) % points.len()];

        x += (current_element.y - next_element.y) * (current_element.z + next_element.z);
        y += (current_element.z - next_element.z) * (current_element.x + next_element.x);
        z += (current_element.x - next_element.x) * (current_element.y + next_element.y);
    }
    let norm = Vec3 { x, y, z };
    let norm = norm.normalize();
    norm
    */

    // Rustic version of the code
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .fold(
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            |acc, (cur, next)| Vec3 {
                x: acc.x + (cur.y - next.y) * (cur.z + next.z),
                y: acc.y + (cur.z - next.z) * (cur.x + next.x),
                z: acc.z + (cur.x - next.x) * (cur.y + next.y),
            },
        )
        .normalize()
}
/// Generates a tri
/// a--d
/// |  |
/// b--c
pub fn tri(a: Vec3, b: Vec3, c: Vec3) -> [Vertex; 3] {
    // Calculate normal using newell method
    //let norm = &vec3(0.0, 0.0, 0.0) - ((&c - a).cross(&b - a));
    let norm = newell(vec![a, b, c]);

    [vertex(a, norm), vertex(b, norm), vertex(c, norm)]
}
// Helper functions
/// Converts quad to tris
/// a--d
/// |  |
/// b--c
pub fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> [Vertex; 6] {
    // Calculate normal using newell method
    let norm = newell(vec![a, b, c, d]);

    //let norm = &vec3(0.0, 0.0, 0.0) - ((&d - a).cross(&b - a));

    [
        vertex(a, norm),
        vertex(b, norm),
        vertex(c, norm),
        vertex(c, norm),
        vertex(d, norm),
        vertex(a, norm),
    ]
}

pub fn polygon(vertices: &[Vec3]) -> Vec<Vertex> {
    vertices
        .windows(3)
        .flat_map(|vertices| {
            let norm = newell(vec![vertices[0], vertices[1], vertices[2]]);
            vec![
                vertex(vertices[0], norm),
                vertex(vertices[1], norm),
                vertex(vertices[2], norm),
            ]
        })
        .collect()
}

pub fn star(num_points: u16, in_radius: f32, out_radius: f32) -> Vec<Vec3> {
    let theta = PI / f32::from(num_points);

    (0..=num_points)
        .flat_map(|i| {
            let i = f32::from(i);
            vec![
                vec3(
                    in_radius * (i * theta * 2.0).cos(),
                    0.0,
                    in_radius * (i * theta * 2.0).sin(),
                ),
                vec3(
                    out_radius * (i * theta * 2.0 + 1.0).cos(),
                    0.0,
                    out_radius * (i * theta * 2.0 + 1.0).sin(),
                ),
                vec3(0.0, 0.0, 0.0),
            ]
        })
        .collect()
}
/// Generates a rectangular_prism, cen
pub fn rectangular_prism(center: Vec3, width: f32, height: f32, depth: f32) -> Vec<Vertex> {
    // Easy access to self elements
    // Start by creating the table top
    // FRONT
    // (view from front)
    // ftl--ftr
    // |      |
    // fbl--fbr
    let ftl = vec3(-width / 2.0, depth / 2.0, -height / 2.0);
    let fbl = vec3(-width / 2.0, -depth / 2.0, -height / 2.0);
    let fbr = vec3(width / 2.0, -depth / 2.0, -height / 2.0);
    let ftr = vec3(width / 2.0, depth / 2.0, -height / 2.0);
    // BACK
    // (view from front)
    // btl--btr
    // |      |
    // bbl--bbr
    let btl = vec3(-width / 2.0, depth / 2.0, height / 2.0);
    let bbl = vec3(-width / 2.0, -depth / 2.0, height / 2.0);
    let bbr = vec3(width / 2.0, -depth / 2.0, height / 2.0);
    let btr = vec3(width / 2.0, depth / 2.0, height / 2.0);
    // Allocate vector for each quad
    let mut vertices: Vec<Vertex> = Vec::with_capacity(36);
    // Front
    vertices.extend_from_slice(&quad(ftl, fbl, fbr, ftr));
    // Back
    vertices.extend_from_slice(&quad(btr, bbr, bbl, btl));
    // Left
    vertices.extend_from_slice(&quad(btl, bbl, fbl, ftl));
    // Right
    vertices.extend_from_slice(&quad(ftr, fbr, bbr, btr));
    // Top
    vertices.extend_from_slice(&quad(btl, ftl, ftr, btr));
    // Bottom
    vertices.extend_from_slice(&quad(fbl, bbl, bbr, fbr));

    vertices
        .iter()
        .map(
            |Vertex {
                 position,
                 normal,
                 texture,
             }| Vertex {
                position: position + center,
                normal: *normal,
                texture: *texture,
            },
        )
        .collect()
}

pub fn get_tex_const(index: u8) -> GLenum {
    match index {
        0 => gl::TEXTURE0,
        1 => gl::TEXTURE1,
        2 => gl::TEXTURE2,
        3 => gl::TEXTURE3,
        4 => gl::TEXTURE4,
        5 => gl::TEXTURE5,
        6 => gl::TEXTURE6,
        7 => gl::TEXTURE7,
        8 => gl::TEXTURE8,
        9 => gl::TEXTURE9,
        10 => gl::TEXTURE10,
        11 => gl::TEXTURE11,
        12 => gl::TEXTURE12,
        13 => gl::TEXTURE13,
        14 => gl::TEXTURE14,
        15 => gl::TEXTURE15,
        16 => gl::TEXTURE16,
        17 => gl::TEXTURE17,
        18 => gl::TEXTURE18,
        19 => gl::TEXTURE19,
        20 => gl::TEXTURE20,
        21 => gl::TEXTURE21,
        22 => gl::TEXTURE22,
        23 => gl::TEXTURE23,
        24 => gl::TEXTURE24,
        25 => gl::TEXTURE25,
        26 => gl::TEXTURE26,
        27 => gl::TEXTURE27,
        28 => gl::TEXTURE28,
        29 => gl::TEXTURE29,
        30 => gl::TEXTURE30,
        31 => gl::TEXTURE31,
        _ => panic!("Out of textures"),
    }
}
