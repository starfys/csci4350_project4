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

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn normalize(self) -> Vec3 {
        let sum = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        Vec3 {
            x: self.x / sum,
            y: self.y / sum,
            z: self.z / sum,
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

pub type Matrix44 = [f32; 16];

trait Matrix {
    fn get(&self, row: usize, col: usize) -> f32;
    fn set(&mut self, row: usize, col: usize, x: f32);
}
impl Matrix for Matrix44 {
    fn get(&self, row: usize, col: usize) -> f32 {
        self[row * 4 + col]
    }
    fn set(&mut self, row: usize, col: usize, x: f32) {
        self[row * 4 + col] = x
    }
}
pub fn zeros() -> Matrix44 {
    [0f32; 16]
}

pub fn identity() -> Matrix44 {
    let mut matrix = zeros();
    matrix.set(0, 0, 1.0);
    matrix.set(1, 1, 1.0);
    matrix.set(2, 2, 1.0);
    matrix.set(3, 3, 1.0);
    matrix
}
pub fn scale(s_x: f32, s_y: f32, s_z: f32) -> Matrix44 {
    let mut matrix = zeros();
    matrix.set(0, 0, s_x);
    matrix.set(1, 1, s_y);
    matrix.set(2, 2, s_z);
    matrix.set(3, 3, 1.0);
    matrix
}

pub fn rotate_x(theta: f32) -> Matrix44 {
    let mut matrix = identity();
    matrix[5] = theta.cos();
    matrix[6] = theta.sin();
    matrix[9] = -theta.sin();
    matrix[10] = theta.cos();
    matrix
}

pub fn rotate_y(theta: f32) -> Matrix44 {
    let mut matrix = identity();
    matrix[0] = theta.cos();
    matrix[2] = theta.sin();
    matrix[8] = -theta.sin();
    matrix[10] = theta.cos();
    matrix
}

pub fn translate(x: f32, y: f32, z: f32) -> Matrix44 {
    let mut matrix = identity();
    matrix.set(3, 0, x);
    matrix.set(3, 1, y);
    matrix.set(3, 2, z);
    matrix
}

pub fn viewing_matrix(eye: Vec3, up: Vec3, target: Vec3) -> Matrix44 {
    let v = (&target - eye).normalize();
    let n = v.cross(up).normalize();
    let u = n.cross(v).normalize();

    let v = v * -1.0;

    let mut matrix = identity();

    matrix.set(0, 0, n.x);
    matrix.set(1, 0, n.y);
    matrix.set(2, 0, n.z);
    matrix.set(3, 0, -n.dot(&eye));

    matrix.set(0, 1, u.x);
    matrix.set(1, 1, u.y);
    matrix.set(2, 1, u.z);
    matrix.set(3, 1, -u.dot(&eye));

    matrix.set(0, 2, v.x);
    matrix.set(1, 2, v.y);
    matrix.set(2, 2, v.z);
    matrix.set(3, 2, -v.dot(&eye));
    matrix
}

pub fn orthogonal_matrix(
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    near: f32,
    far: f32,
) -> Matrix44 {
    // Start with zeroes
    let mut matrix = zeros();

    // Calculate some frequently used values
    let w = right - left;
    let h = top - bottom;
    let d = far - near;

    matrix.set(0, 0, 2.0 / w);
    matrix.set(1, 1, 2.0 / h);
    matrix.set(2, 2, -2.0 / d);
    matrix.set(3, 0, -(right + left) / w);
    matrix.set(3, 1, -(top + bottom) / h);
    matrix.set(3, 2, -(far + near) / d);
    matrix.set(3, 3, 1.0);
    matrix
}

pub fn perspective_matrix(fov: f32, aspect: f32, near: f32, far: f32) -> Matrix44 {
    let mut matrix = zeros();
    matrix[0] = 1.0 / fov.tan() / aspect;
    matrix[5] = 1.0 / fov.tan();
    matrix[10] = -(far + near) / (far - near);
    matrix[11] = -1.0;
    matrix[14] = -2.0 * far * near / (far - near);
    matrix
}

pub fn matmul(a: Matrix44, b: Matrix44) -> Matrix44 {
    let mut c = zeros();
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                c[i * 4 + j] += a.get(i, k) * b.get(k, j);
            }
        }
    }
    c
}
