use std::error::Error;
use std::f32::consts::PI;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use gleam::gl;
use gleam::gl::types::{GLint, GLsizei};
use image::GenericImageView;

use super::Context;
use error::io_error;
use matrix::{identity, matmul, rotate_x, rotate_y, scale, translate, vec2, vec3, Vec2, Vec3};
use render::{get_tex_const, Color, Drawable};

#[derive(Debug)]
pub struct Face<T> {
    indices: Vec<FaceIndex<T>>,
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
            .map(|token| token.parse::<T>().ok())
            .unwrap_or(None);
        let normal_index: Option<T> = tokens
            .next()
            .map(|token| token.parse::<T>().ok())
            .unwrap_or(None);
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
    pub faces: Vec<Face<u32>>,
}
impl Group {
    pub fn new(name: &str) -> Self {
        Group {
            name: name.into(),
            faces: Vec::new(),
        }
    }
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
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texture_coords: Vec<Vec2>,
    center: Vec3,
    scale: Vec3,
    translate: Vec3,
    texture_path: PathBuf,
    cur_texture: u8,
}
impl Obj {
    /// Loads a render object from a path
    pub fn load<P, PP>(
        obj_path: P,
        texture_path: PP,
        cur_texture: &mut u8,
        scale: Vec3,
        translate: Vec3,
    ) -> Result<Self, io::Error>
    where
        P: AsRef<Path> + std::fmt::Display,
        PP: AsRef<OsStr> + Sized,
    {
        // Get the path as string for later
        let path_str = obj_path.to_string();
        // Read the obj file
        let obj_file = File::open(obj_path)?;
        // Create reader for the file
        let obj_file = BufReader::new(obj_file);
        // Buffers for data
        let mut vertices: Vec<Vec3> = Vec::new();
        let mut normals: Vec<Vec3> = Vec::new();
        let mut texture_coords: Vec<Vec2> = Vec::new();
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
                    if !cur_group.faces.is_empty() {
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
                    vertices.push(v);
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
                    normals.push(vec3(x, y, z));
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
                    texture_coords.push(vec2(x, y));
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
        println!("Center for {} is {:?}", path_str, center);
        // Iterate texture counter forward
        *cur_texture += 1;
        // Generate the render object
        Ok(Obj {
            groups,
            vert_start: 0,
            num_verts: 0,
            vertices,
            normals,
            texture_coords,
            center,
            scale,
            translate,
            texture_path: Path::new(&texture_path).to_path_buf(),
            cur_texture: *cur_texture,
        })
    }

    pub fn to_vertices(&self, group: &Group) -> Vec<f32> {
        // Generate vertex list from face list
        group
            .faces
            .iter()
            // For each face, get the vertex, normal, and texture coordinates
            // of all its components
            .flat_map(|face| {
                face.indices.iter().map(|index| {
                    (
                        // Get the vertex for this
                        /*(&(&self.vertices[(index.vertex_index - 1) as usize] - self.center)
                        + self.translate)
                        .scale(self.scale.x, self.scale.y, self.scale.z),*/
                        // Get the vertex for this
                        &self.vertices[(index.vertex_index - 1) as usize] - self.center,
                        index
                            .normal_index
                            .map(|normal_index| self.normals[(normal_index - 1) as usize])
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
impl Drawable for Obj {
    /// Returns buffer data
    fn buffer_data(&mut self, vertex_start: GLint) -> Vec<f32> {
        // Store element start
        self.vert_start = vertex_start;
        // Store vertex data
        let mut vertices: Vec<f32> = Vec::new();
        // Iterate over groups
        for group in &self.groups {
            // Extract data for the current group
            let cur_vertices = self.to_vertices(group);
            // Add existing data
            vertices.extend_from_slice(&cur_vertices);
        }
        // Store the number of vertices
        self.num_verts = (vertices.len() / 8) as GLsizei;
        // Return vertices
        vertices
    }
    /// Loads textures
    fn load_texture(&self, ctx: &Context) {
        let gl = &ctx.gl;
        // Read texture
        let tex_image = image::open(self.texture_path.clone()).unwrap();

        // Extract dimensions
        let (width, height) = tex_image.dimensions();
        // Get image as raw bytes
        let tex_image = tex_image.as_rgb8().unwrap().clone();
        // Create a texture
        let texture = gl.gen_textures(1)[0];
        // Get the texture index as a glenum
        let tex_enum = get_tex_const(self.cur_texture);
        gl.active_texture(tex_enum);
        gl.bind_texture(gl::TEXTURE_2D, texture);
        gl.tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            width as i32,
            height as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            Some(&tex_image),
        );
        gl.generate_mipmap(gl::TEXTURE_2D);

        gl.tex_parameter_i(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
    }
    /// Draws the object
    // Return groups
    fn draw(&self, ctx: &Context) {
        let gl = &ctx.gl;
        let mv_location = gl.get_uniform_location(ctx.program, "uMVMatrix");
        let m_matrix = identity();
        let v_matrix = matmul(
            rotate_y(PI),
            matmul(
                scale(self.scale.x, self.scale.y, self.scale.z),
                matmul(
                    translate(self.translate.x, self.translate.y, self.translate.z),
                    ctx.camera,
                ),
            ),
        );
        let mv_matrix = matmul(v_matrix, m_matrix);
        gl.uniform_matrix_4fv(mv_location, false, &mv_matrix);

        let sampler_location = gl.get_uniform_location(ctx.program, "uSampler");
        gl.uniform_1i(sampler_location, self.cur_texture as i32);

        // Lighting properties
        let ambient_location = gl.get_uniform_location(ctx.program, "uAmbientProduct");
        let diffuse_location = gl.get_uniform_location(ctx.program, "uDiffuseProduct");
        let specular_location = gl.get_uniform_location(ctx.program, "uSpecularProduct");
        // Light position
        let shininess_location = gl.get_uniform_location(ctx.program, "uShininess");

        gl.uniform_4f(ambient_location, 0.4, 0.8, 0.8, 1.0);
        gl.uniform_4f(diffuse_location, 0.75164, 0.60648, 0.22648, 1.0);
        gl.uniform_4f(specular_location, 0.628281, 0.555802, 0.366065, 1.0);

        gl.uniform_1f(shininess_location, 0.4 * 128.0);

        gl.draw_arrays(gl::TRIANGLES, self.vert_start / 8, self.num_verts);
    }
}
