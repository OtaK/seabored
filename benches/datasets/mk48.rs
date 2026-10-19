use crate::datasets::HasSample;

pub const BYTES: &[u8] = include_bytes!("cbor/mk48.cbor");
pub const ID: &str = "mk48";

#[derive(serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum EntityType {
    ArleighBurke,
    Bismarck,
    Clemenceau,
    Fletcher,
    G5,
    Iowa,
    Kolkata,
    Osa,
    Yasen,
    Zubr,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Transform {
    pub altitude: i8,
    pub angle: u16,
    pub position: (f32, f32),
    pub velocity: i16,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Guidance {
    pub angle: u16,
    pub submerge: bool,
    pub velocity: i16,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Contact {
    pub damage: u8,
    pub entity_id: u32,
    pub entity_type: Option<EntityType>,
    pub guidance: Guidance,
    pub player_id: Option<u16>,
    pub reloads: Vec<bool>,
    pub transform: Transform,
    pub turret_angles: Vec<u16>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TerrainUpdate {
    pub chunk_id: (i8, i8),
    pub data: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Update {
    pub contacts: Vec<Contact>,
    pub score: u32,
    pub world_radius: f32,
    pub terrain_updates: Vec<TerrainUpdate>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Updates {
    pub updates: Vec<Update>,
}

impl HasSample for Updates {
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}
