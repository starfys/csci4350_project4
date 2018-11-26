use std::collections::HashMap;
use std::f32::consts::PI;
use std::io;
use std::path::Path;

use gleam::gl;
use gleam::gl::types::{GLint, GLsizei};
use rand::Rng;

use super::{Context, U32_SIZE};
use error::io_error;
use matrix::{identity, matmul, rotate_x, rotate_y, scale, translate, vec3, Vec2, Vec3};

pub trait Drawable {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32>;
    /// Draws the shape
    fn draw(&self, gl: &Context);
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

// Helper functions
/// Converts quad to tris
/// a--d
/// |  |
/// b--c
pub fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> [Vertex; 6] {
    // Calculate normal from a corner
    let norm = &vec3(0.0, 0.0, 0.0) - ((&d - a).cross((&b - a)));

    [
        vertex(a, norm),
        vertex(b, norm),
        vertex(c, norm),
        vertex(c, norm),
        vertex(d, norm),
        vertex(a, norm),
    ]
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
        ).collect()
}
