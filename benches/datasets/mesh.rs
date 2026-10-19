use crate::datasets::HasSample;

pub const BYTES: &[u8] = include_bytes!("cbor/mesh.cbor");
pub const ID: &str = "mesh";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Triangle {
    pub v0: Vector3,
    pub v1: Vector3,
    pub v2: Vector3,
    pub normal: Vector3,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
}

impl HasSample for Mesh {
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}
