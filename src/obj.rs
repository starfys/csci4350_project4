use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use gleam::gl;
use gleam::gl::types::{GLint, GLsizei};
use rand::Rng;

use super::Context;
use error::io_error;
use matrix::{identity, matmul, rotate_x, rotate_y, scale, translate};
use render::{Color, Drawable};

#[derive(Copy, Clone, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl<'a> std::ops::Add<Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Self::Output {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}
impl<'a> std::ops::Sub<Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Self::Output {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, other: f32) -> Self::Output {
        Vec3 {
            x: other * self.x,
            y: other * self.y,
            z: other * self.z,
        }
    }
}

pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}
impl Vec3 {
    pub fn origin() -> Vec3 {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
pub fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}
impl Vec2 {
    pub fn origin() -> Vec2 {
        Vec2 { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug)]
pub struct Face<T> {
    indices: Vec<FaceIndex<T>>,
}
impl<T> Face<T>
where
    T: Clone,
{
    fn to_triangles(&self) -> Vec<T> {
        if self.indices.len() == 4 {
            [
                &self.indices[0],
                &self.indices[1],
                &self.indices[2],
                &self.indices[2],
                &self.indices[3],
                &self.indices[0],
            ]
                .iter()
                .map(|face_index| face_index.vertex_index.clone())
                .collect()
        } else {
            self.indices
                .windows(3)
                .flatten()
                .map(|face_index| face_index.vertex_index.clone())
                .collect()
        }
    }
}
fn face<T>(indices: Vec<FaceIndex<T>>) -> Face<T> {
    Face { indices }
}
#[derive(Debug)]
pub struct FaceIndex<T> {
    vertex_index: T,
    texture_index: Option<T>,
    normal_index: Option<T>,
}

impl<T> FromStr for FaceIndex<T>
where
    T: FromStr + Default,
    <T as FromStr>::Err: 'static + Error + Send + Sync,
{
    type Err = io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split('/');
        // Get vertex index
        let vertex_index: T = tokens
            .next()
            .ok_or_else(|| io_error("Missing vertex index"))?
            .parse()
            .map_err(io_error)?;
        let texture_index: Option<T> = tokens
            .next()
            .map(|token| token.parse::<T>().unwrap_or_default());
        let normal_index: Option<T> = tokens
            .next()
            .map(|token| token.parse::<T>().unwrap_or_default());
        Ok(FaceIndex {
            vertex_index,
            texture_index,
            normal_index,
        })
    }
}

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texture_coords: Vec<Vec2>,
    pub faces: Vec<Face<u32>>,
}
impl Group {
    pub fn new(name: &str) -> Self {
        Group {
            name: name.into(),
            vertices: Vec::new(),
            normals: Vec::new(),
            texture_coords: Vec::new(),
            faces: Vec::new(),
        }
    }
    pub fn to_vertices(&self, center: Option<Vec3>) -> Vec<f32> {
        // If center is not given, just use origin
        let center = center.unwrap_or_else(Vec3::origin);
        // Generate vertex list from face list
        self
            .faces
            .iter()
            // For each face, get the vertex, normal, and texture coordinates
            // of all its components
            .flat_map(|face| {
                face.indices.iter().map(|index| {
                    (
                        // Get the vertex for this
                        &self.vertices[(index.vertex_index - 1) as usize] - center,
                        index
                            .normal_index
                            .map(|normal_index| self.normals[(normal_index  - 1) as usize])
                            .unwrap_or_else(Vec3::origin),
                        index
                            .texture_index
                            .map(|texture_index| self.texture_coords[(texture_index - 1) as usize])
                            .unwrap_or_else(Vec2::origin),
                    )
                })
            })
            // Flatten out everything
            .flat_map(|(vertex, normal, texture)| {
                #[cfg_attr(rustfmt, rustfmt_skip)] 
                vec![
                    vertex.x, vertex.y, vertex.z,
                    normal.x, normal.y, normal.z,
                    texture.x, texture.y,
                ]
            })
            .collect()
    }
}

pub struct DrawInfo<T> {
    pub vertices: Vec<f32>,
    pub indices: Vec<T>,
}

impl<T> DrawInfo<T> {
    fn empty() -> Self {
        DrawInfo {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<(Vec<Group>, Vec3), io::Error> {
    // Read the obj file
    let obj_file = File::open(path)?;
    // Create reader for the file
    let obj_file = BufReader::new(obj_file);
    // Create list of groups
    let mut groups: Vec<Group> = Vec::new();
    // current group
    let mut cur_group: Group = Group::new("");
    // Keep track of center
    let mut center: Vec3 = Vec3::origin();
    // Keep track of vertices for averaging center
    // Float is used here for division
    let mut num_vertices: f32 = 0.0;

    for line in obj_file.lines() {
        // Unwrap the line
        let line = line?;
        // Ignore comments
        if line.starts_with('#') {
            continue;
        }
        // Split line into tokens
        let mut tokens = line.split_whitespace();
        // Read the first token
        let ty = match tokens.next() {
            Some(token) => token,
            // Skip empty lines
            None => {
                continue;
            }
        };
        // Handle it
        match ty {
            "g" => {
                // Read group name
                let name = tokens.next().unwrap_or("unnamed");
                // Insert old group into groups
                if !cur_group.vertices.is_empty() {
                    groups.push(cur_group);
                }
                // Create new group
                cur_group = Group::new(name);
            }
            "v" => {
                // Read coordinates
                let x: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                let y: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                let z: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                // Collect into a vector
                let v = vec3(x, y, z);
                // Factor vertex into the center
                center = &center + v;
                // Add to number of vertices
                num_vertices += 1.0;
                // Add vector into the list
                cur_group.vertices.push(v);
            }
            "vn" => {
                // Read coordinates
                let x: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                let y: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                let z: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                cur_group.normals.push(vec3(x, y, z));
            }
            "vt" => {
                // Read coordinates
                let x: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                let y: f32 = tokens
                    .next()
                    .unwrap_or_else(|| "0")
                    .parse()
                    .unwrap_or_else(|_| 0.0);
                cur_group.texture_coords.push(vec2(x, y));
            }
            "f" => {
                let face_indices = tokens.map(FaceIndex::from_str).flatten().collect();
                cur_group.faces.push(face(face_indices));
            }
            other => {
                eprintln!("Unhandled line type: {}", other);
            }
        }
    }
    // Push the last group
    groups.push(cur_group);
    // Average out the center
    let center = center * (1.0 / (num_vertices as f32));
    // Return groups
    Ok((groups, center))
}


struct Material {
    /// Ka
    ambient_color: Color,
    /// Kd
    diffuse_color: Color,
    /// Ks
    specular_color: Color,
    /// Ns 
    specular_exponent: f32,
    /// Ni
    optical_density: f32,
    /// d or Tr
    transparency: f32,
    // TODO: illum 
    // TODO: maps
}

pub struct Obj {
    groups: Vec<Group>,
    vert_start: GLint,
    num_verts: GLsizei,
    center: Vec3,
    translate: Vec3,
}

impl Obj {
    /// Loads a render object from a path
    pub fn load<P: AsRef<Path>>(path: P, translate: Vec3) -> Result<Self, io::Error> {
        // Parse object file
        let (groups, center) = load_obj(path)?;
        // Generate the render object
        Ok(Obj {
            groups,
            vert_start: 0,
            num_verts: 0,
            center,
            translate,
        })
    }
}
impl Drawable for Obj {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        // Store element start
        self.vert_start = vertex_start;
        // Store vertex data
        let mut vertices: Vec<f32> = Vec::new();
        // Store index data
        let _indices: Vec<u32> = Vec::new();
        // Iterate over groups
        for group in &self.groups {
            // Extract data for the current group
            let cur_vertices = group.to_vertices(Some(self.center));
            // Add existing data
            vertices.extend_from_slice(&cur_vertices);
        }
        // Store the number of vertices
        self.num_verts = vertices.len() as GLsizei;
        // Return vertices
        vertices
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

        // Lighting properties
        let ambient_location = gl.get_uniform_location(ctx.program, "uAmbientProduct");
        let diffuse_location = gl.get_uniform_location(ctx.program, "uDiffuseProduct");
        let specular_location = gl.get_uniform_location(ctx.program, "uSpecularProduct");
        // Light position
        let light_position_location = gl.get_uniform_location(ctx.program, "uLightPosition");
        let shininess_location = gl.get_uniform_location(ctx.program, "uShininess");

        // Set lighting properties
        gl.uniform_4f(ambient_location, 0.0, 0.0, 0.0, 1.0);
        gl.uniform_4f(diffuse_location, 0.64, 0.64, 0.64, 1.0);
        gl.uniform_4f(specular_location, 0.0, 0.0, 0.0, 1.0);

        gl.uniform_4f(light_position_location, 0.0, 1.0, 0.0, 1.0);

        gl.uniform_1f(shininess_location, 96.078431);

        gl.draw_arrays(gl::TRIANGLES, self.vert_start, self.num_verts / 8);
    }
}
