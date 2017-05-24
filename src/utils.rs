extern crate genmesh;
extern crate obj;

use std::ops::{Add, Mul};
use std::iter::Sum;
use std::f32::consts::PI;

use types::{ RenderItem, RenderItemBuilder, TransformBuilder };
use types::{ Vertex, Quaternion, Vector3, Matrix4, CamState, MaterialBuilder };


/// Returns a Vec<Vertex> that should be converted to buffer and rendered as `TrianglesList`.
pub fn load_wavefront( data: &[u8]) -> Vec<Vertex> {
    let mut data = ::std::io::BufReader::new(data);
    let data = obj::Obj::load(&mut data);

    let mut vertex_data = Vec::new();

    for shape in data.object_iter().next().unwrap().group_iter().flat_map(|g| g.indices().iter()) {
        match shape {
            &genmesh::Polygon::PolyTri(genmesh::Triangle { x: v1, y: v2, z: v3 }) => {
                for v in [v1, v2, v3].iter() {
                    let position = data.position()[v.0];
                    let texture = v.1.map(|index| data.texture()[index]);
                    let normal = v.2.map(|index| data.normal()[index]);

                    let texture = texture.unwrap_or([0.0, 0.0]);
                    let normal = normal.unwrap_or([0.0, 0.0, 0.0]);

                    vertex_data.push(Vertex {
                        position: position,
                        normal: normal,
                        texture: texture,
                    })
                }
            },
            _ => unimplemented!()
        }
    }

    vertex_data
}


/// Returns a RenderItem for the skydome
pub fn create_skydome(shader_name: &'static str) -> RenderItem {
    RenderItemBuilder::default()
        .name("skydome".to_string())
        .vertices(load_wavefront(include_bytes!("./resources/skydome.obj")))
        .material(MaterialBuilder::default()
                  .shader_name(shader_name.to_string())
                  .build()
                  .unwrap())
        .instance_transforms(vec![
                             TransformBuilder::default()
                             .scale((300f32, 300f32, 300f32))
                             .build()
                             .unwrap()
        ])
        .build()
        .unwrap()
}

/// Returns the dot product of two vectors
pub fn dotp<T>(this: &[T], other: &[T]) -> T where T:Add<T, Output=T> + Mul<T, Output=T> + Sum + Copy {
    assert!(this.len() == other.len(), "The dimensions must be equal");

    this.iter().zip(other.iter())
        .map(|(&a, &b)| a * b)
        .sum()
}

/// returns the cross product of two vectors
pub fn crossp(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [(a[1] * b[2]) - (a[2] * b[1]), (a[2] * b[0]) - (a[0] * b[2]), (a[0] * b[1]) - (a[1] * b[0])]
}

/// returns the resultant vector of a - b
pub fn sub_vec3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// returns the normal calculated from the three vectors supplied
pub fn calc_normal(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> [f32; 3] {
    let a = sub_vec3(p1, p0);
    let b = sub_vec3(p2, p0);

    crossp(a, b)
}

/// returns the two matrices multiplied together
pub fn mul_mat4(a: Matrix4, b: Matrix4) -> Matrix4 {
    let mul_vec = a.iter().zip(b.iter())
        .map(|(&a, &b)| {
            a.iter().zip(b.iter()).map(|(&c, &d)| c * d).collect()
        }).collect::<Vec<Vec<f32>>>();

    [[mul_vec[0][0], mul_vec[0][1], mul_vec[0][2], mul_vec[0][3]],
    [mul_vec[1][0], mul_vec[1][1], mul_vec[1][2], mul_vec[1][3]],
    [mul_vec[2][0], mul_vec[2][1], mul_vec[2][2], mul_vec[2][3]],
    [mul_vec[3][0], mul_vec[3][1], mul_vec[3][2], mul_vec[3][3]]]
}

/// returns a euler angle as a quaternion
pub fn to_quaternion(angle: Vector3) -> Quaternion {
    let (c3, c1, c2) = ((angle.0 / 2f32).cos(), (angle.1 / 2f32).cos(), (angle.2 / 2f32).cos());
    let (s3, s1, s2) = ((angle.0 / 2f32).sin(), (angle.1 / 2f32).sin(), (angle.2 / 2f32).sin());

    let c1c2 = c1 * c2;
    let s1s2 = s1 * s2;
    let w = c1c2 * c3 - s1s2 * s3;
    let x = c1c2 * s3 + s1s2 * c3;
    let y = s1 * c2 * c3 + c1 * s2 * s3;
    let z = c1 * s2 * c3 - s1 * c2 * s3;

    (x, y, z, w)
}

/// returns a quaternion from a euler angle
pub fn to_euler(angle: Quaternion) -> Vector3 {
    let ysqr = angle.1 * angle.1;
    let t0 = -2.0f32 * (ysqr + angle.2 * angle.2) + 1.0f32;
    let t1 = 2.0f32 * (angle.0 * angle.1 - angle.3 * angle.2);
    let mut t2 = -2.0f32 * (angle.0 * angle.2 + angle.3 * angle.1);
    let t3 = 2.0f32 * (angle.1 * angle.2 - angle.3 * angle.0);
    let t4 = -2.0f32 * (angle.0 * angle.0 + ysqr) + 1.0f32;

    t2 = if t2 > 1.0f32 { 1.0f32 } else { t2 };
    t2 = if t2 < -1.0f32 { -1.0f32 } else { t2 };

    let pitch = t2.asin();
    let roll = t3.atan2(t4);
    let yaw = t1.atan2(t0);

    (pitch, roll, yaw)
}

/// Returns perspective projection matrix given fov, aspect ratio, z near and far
pub fn build_persp_proj_mat(fov:f32,aspect:f32,znear:f32,zfar:f32) -> Matrix4 {
    let ymax = znear * (fov * (PI/360.0)).tan();
    let ymin = -ymax;
    let xmax = ymax * aspect;
    let xmin = ymin * aspect;

    let width = xmax - xmin;
    let height = ymax - ymin;

    let depth = zfar - znear;
    let q = -(zfar + znear) / depth;
    let qn = -2.0 * (zfar * znear) / depth;

    let w = 2.0 * znear / width;
    let h = 2.0 * znear / height;

    [[w, 0.0f32, 0.0f32, 0.0f32],
    [0.0f32, h, 0.0f32, 0.0f32],
    [0.0f32, 0.0f32, q, -1.0f32],
    [0.0f32, 0.0f32, qn, 0.0f32]]
}

/// Returns the model view matrix for a first person view given cam position and rotation
pub fn build_fp_view_matrix(cam_state: &CamState) -> Matrix4 {

    let (sin_yaw, cos_yaw, sin_pitch, cos_pitch) = (
        cam_state.cam_rot.1.sin(),
        cam_state.cam_rot.1.cos(),
        cam_state.cam_rot.0.sin(),
        cam_state.cam_rot.0.cos());
    let xaxis = [cos_yaw, 0.0, -sin_yaw];
    let yaxis = [sin_yaw * sin_pitch, cos_pitch, cos_yaw * sin_pitch];
    let zaxis = [sin_yaw * cos_pitch, -sin_pitch, cos_pitch * cos_yaw];

    let cam_arr = [cam_state.cam_pos.0, cam_state.cam_pos.1, cam_state.cam_pos.2];

    [[ xaxis[0], yaxis[0], zaxis[0], 0.0],
    [ xaxis[1], yaxis[1], zaxis[1], 0.0],
    [ xaxis[2], yaxis[2], zaxis[2], 0.0],
    [ -dotp(&xaxis, &cam_arr), -dotp(&yaxis, &cam_arr), -dotp(&zaxis, &cam_arr), 1.0f32]]
}
