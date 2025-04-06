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
pub fn gen_cube(size: i32,pos:Vec3,col: Vec3) -> Mesh<Vertex> {
    let mut mesh = Mesh::new();
    for dir in utils::DIRECTIONS {
        for v in chunk::gen_voxel_face(dir,0.,0.,0.) {
            mesh.verts.push( vertex::Vertex {
                pos: vec3!(v) * size as f32 + pos,
                norm: dir,
                col: col,
            });
        }
    }
    mesh.indices = chunk::gen_indices(mesh.verts.len());
    return mesh;
}
pub fn gen_icosahedron(size: f32,pos: Vec3,col: Vec3) -> Mesh<Vertex> {
    let mut mesh = Mesh::new();

    let phi = (1.0 + f32::sqrt(5.0)) * 0.5; // golden ratio
    let a = size;
    let b = size / phi;

    // add vertices
    mesh.verts.extend_from_slice(&[
        Vertex { pos: vec3!(0., b, -a)+pos,    norm: vec3!(0.,b,-a),      col},
        Vertex { pos: vec3!(b, a, 0.)+pos,     norm: vec3!(b, a, 0.),     col},
        Vertex { pos: vec3!(-b, a, 0.)+pos,    norm: vec3!(-b, a, 0.),    col},
        Vertex { pos: vec3!(0., b, a)+pos,     norm: vec3!(0., b, a),     col},
        Vertex { pos: vec3!(0., -b, a)+pos,    norm: vec3!(0., -b, a),    col},
        Vertex { pos: vec3!(-a, 0., b)+pos,    norm: vec3!(-a, 0., b),    col},
        Vertex { pos: vec3!(0., -b, -a)+pos,   norm: vec3!(0., -b, -a),   col},
        Vertex { pos: vec3!(a, 0., -b)+pos,    norm: vec3!(a, 0., -b),    col},
        Vertex { pos: vec3!(a, 0., b)+pos,     norm: vec3!(a, 0., b),     col},
        Vertex { pos: vec3!(-a, 0., -b)+pos,   norm: vec3!(-a, 0., -b),   col},
        Vertex { pos: vec3!(b, -a, 0.)+pos,    norm: vec3!(b, -a, 0.),    col},
        Vertex { pos: vec3!(-b, -a, 0.)+pos,   norm: vec3!(-b, -a, 0.),   col} ]);

    // normalise the normals
    for i in 0..mesh.verts.len() {
        mesh.verts[i].norm = mesh.verts[i].norm.norm();
    }

    // add triangles
    mesh.indices.extend_from_slice(&[
     2,  1,  0,   1,  2, 3,   5,  4,  3,  
     4,  8,  3,   7,  6, 0,   6,  9,  0,  
    11, 10,  4,  10, 11, 6,   9,  5,  2,  
     5,  9, 11,   8,  7, 1,   7,  8, 10,  
     2,  5,  3,   8,  1, 3,   9,  2,  0,  
     1,  7,  0,  11,  9, 6,   7, 10,  6,  
     5,  11, 4,  10,  8, 4  ]);

    return mesh;
}
