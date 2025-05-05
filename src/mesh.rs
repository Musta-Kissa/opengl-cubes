use my_math::prelude::*;
use crate::utils;
use crate::chunk;
use crate::vertex;
use crate::vertex::*;

pub struct Mesh<T> {
    pub verts: Vec<T>,
    pub indices: Vec<u32>,
}
impl<T: VertexAttributes + Clone> Mesh<T> {
    pub fn new() -> Mesh<T> {
        Mesh {
            verts: Vec::new(),
            indices: Vec::new(),
        }
    }
    pub fn join_with(&mut self,other:&Mesh<T>) {
        self.indices.extend_from_slice(other.indices
                                            .as_slice()
                                            .iter()
                                            .map(|i| i + self.verts.len() as u32)
                                            .collect::<Vec<u32>>()
                                            .as_slice()
                                            );
        self.verts.extend_from_slice(&other.verts);
    }
}
pub fn gen_cube_skeleton(size: i32,pos:IVec3) -> Mesh<Vertex> {
    let mut mesh = Mesh::new();
    let size = size as f32;
    let pos:Vec3 = pos.into();
    let positions = [
        vec3!(pos.x,pos.y,pos.z),
        vec3!(pos.x,pos.y,pos.z + size),
        vec3!(pos.x,pos.y + size ,pos.z),
        vec3!(pos.x,pos.y + size ,pos.z + size),
        vec3!(pos.x + size ,pos.y,pos.z),
        vec3!(pos.x + size ,pos.y,pos.z + size ),
        vec3!(pos.x + size ,pos.y + size ,pos.z),
        vec3!(pos.x + size ,pos.y + size ,pos.z + size ),
    ];
    for pos in positions {
        mesh.verts.push( vertex::Vertex {
            pos: pos ,
            norm: Vec3::Y,
            col: vec3!(1.,1.,1.).norm(),
        });
    }
    mesh.indices = vec![ 0,1, 1,3, 3,2, 2,0, 
                         4,5, 5,7, 7,6, 6,4,
                         0,4, 2,6, 1,5, 3,7,];
    return mesh;
}
