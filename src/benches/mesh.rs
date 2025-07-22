use alloc::vec::Vec;
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Facet, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    r: u8,
    g: u8,
    b: u8,
}

impl Vertex {
    pub fn new(i: usize) -> Self {
        Self {
            x: i as f32,
            y: i as f32,
            z: i as f32,
            r: i as u8,
            g: i as u8,
            b: i as u8,
        }
    }
}

pub type Mesh = Vec<Vertex>;

fn mesh(n: usize) -> Mesh {
    (0..n).map(Vertex::new).collect()
}

pub fn mesh_one() -> Mesh {
    mesh(1)
}

pub fn mesh_1k() -> Mesh {
    mesh(1000)
}
