#![allow(dead_code)]
pub mod log;
pub mod mesh;
pub mod mimi_content_multipart_3;
pub mod minecraft_savedata;
pub mod mk48;

pub const SAMPLES: [(&str, &[u8]); 5] = [
    (
        mimi_content_multipart_3::ID,
        mimi_content_multipart_3::BYTES,
    ),
    (log::ID, log::BYTES),
    (mesh::ID, mesh::BYTES),
    (minecraft_savedata::ID, minecraft_savedata::BYTES),
    (mk48::ID, mk48::BYTES),
];

pub fn sample_ids() -> impl Iterator<Item = &'static str> {
    [
        mimi_content_multipart_3::ID,
        log::ID,
        mesh::ID,
        minecraft_savedata::ID,
        mk48::ID,
    ]
    .into_iter()
}

pub trait HasSample {
    fn sample() -> &'static [u8];
}
