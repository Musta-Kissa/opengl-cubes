use crate::mesh::Mesh;
use crate::vertex::Vertex;

use my_math::vec::*;

use std::mem::MaybeUninit;
use std::time::Instant;

use crate::utils::DIRECTIONS;

use fast_noise_lite_rs::{FastNoiseLite, NoiseType};


//const CHUNK_SIZE: usize = 32;
pub const SEED: u64 = 1111;
pub const SIZE: usize = 256;

pub type ChunkData = [[[bool; SIZE]; SIZE]; SIZE];

pub fn gen_chunk_data() -> Box<ChunkData> {
    let start = Instant::now();

    let mut noise = FastNoiseLite::new(SEED as i32);
    noise.set_noise_type(NoiseType::Perlin);
    noise.set_frequency(0.035);

    let has_voxel = |x,y,z| {
        let n = noise.get_noise_3d(
            x as f32 ,
            y as f32 ,
            z as f32 ,
        );
        return n >= 0.
    };

    let mut count = 0;
    let mut chunk_data: Box<[[[MaybeUninit<bool>; SIZE]; SIZE]; SIZE]> = Box::new([[[MaybeUninit::<bool>::uninit();SIZE];SIZE];SIZE]);
    for x in 0..SIZE {
        for y in 0..SIZE {
            for z in 0..SIZE {
                let voxel = has_voxel(x,y,z);
                chunk_data[x as usize][y as usize][z as usize] = MaybeUninit::new(voxel);
                if voxel { count += 1 };
            }
        }
    }
    println!("time: {:?}",start.elapsed());
    println!("cubes: {}",count);
    return unsafe { Box::from_raw(Box::into_raw(chunk_data) as *mut ChunkData) };
}

pub fn gen_mesh(chunk_data: &Box<ChunkData>) -> Mesh {
    let mut vert_pos: Vec<Vertex> = Vec::new();

    for x in 0..SIZE {
        for y in 0..SIZE {
            for z in 0..SIZE {
                if !chunk_data[x][y][z] {
                    continue;
                }
                if x == SIZE-1  || !chunk_data[x+1][y][z] {
                    for v in gen_voxel_face(Vec3::X,x as f32, y as f32, z as f32) {
                        vert_pos.push( Vertex {
                            pos: vec3!(v),
                            norm: DIRECTIONS[0],
                            col: vec3!(0.7,0.,0.),
                        });
                    }
                }
                if x == 0 || !chunk_data[x-1][y][z] {
                    for v in gen_voxel_face(Vec3::NEG_X,x as f32, y as f32, z as f32) {
                        vert_pos.push( Vertex {
                            pos: vec3!(v),
                            norm: DIRECTIONS[1],
                            col: vec3!(0.7,0.,0.),
                        });
                    }
                }
                if y == SIZE-1 || !chunk_data[x][y+1][z] {
                    for v in gen_voxel_face(Vec3::Y,x as f32, y as f32, z as f32) {
                        vert_pos.push( Vertex {
                            pos: vec3!(v),
                            norm: DIRECTIONS[2],
                            col: vec3!(0.7,0.,0.),
                        });
                    }
                }
                if y == 0 || !chunk_data[x][y-1][z] {
                    for v in gen_voxel_face(Vec3::NEG_Y,x as f32, y as f32, z as f32) {
                        vert_pos.push( Vertex {
                            pos: vec3!(v),
                            norm: DIRECTIONS[3],
                            col: vec3!(0.7,0.,0.),
                        });
                    }
                }
                if z == SIZE-1 || !chunk_data[x][y][z+1] {
                    for v in gen_voxel_face(Vec3::Z,x as f32, y as f32, z as f32) {
                        vert_pos.push( Vertex {
                            pos: vec3!(v),
                            norm: DIRECTIONS[4],
                            col: vec3!(0.7,0.,0.),
                        });
                    }
                }
                if z == 0 || !chunk_data[x][y][z-1] {
                    for v in gen_voxel_face(Vec3::NEG_Z,x as f32, y as f32, z as f32) {
                        vert_pos.push( Vertex {
                            pos: vec3!(v),
                            norm: DIRECTIONS[5],
                            col: vec3!(0.7,0.,0.),
                        });
                    }
                }
            }
        }
    }
    println!("size: {}",SIZE);
    println!("triangles: {}",vert_pos.len() / 3);

    Mesh {  indices:gen_indices(vert_pos.len()), verts: vert_pos ,}
}
pub fn gen_indices(len: usize) -> Vec<u32> {
    let mut indices: Vec<u32> = Vec::new();
    indices.reserve_exact(len / 4 * 6);
    //clockwise winding
    for i in (0..len as u32).step_by(4) {
        indices.extend([
            0,1,2,2,3,0
        ].map(|idx| idx + i));
    }
    indices
}
pub fn gen_voxel_face(dir:Vec3,x:f32,y:f32,z:f32) -> [[f32;3];4] {
    match dir {
    // +X
    Vec3::X => 
            [
            [x+1.,y+0.,z+0.],
            [x+1.,y+1.,z+0.],
            [x+1.,y+1.,z+1.],
            [x+1.,y+0.,z+1.],
            ],
    // -X
    Vec3::NEG_X => 
            [
            [x+0.,y+0.,z+0.],
            [x+0.,y+0.,z+1.],
            [x+0.,y+1.,z+1.],
            [x+0.,y+1.,z+0.],
            ],
    // +Y
    Vec3::Y => 
            [
            [x+0.,y+1.,z+0.],
            [x+0.,y+1.,z+1.],
            [x+1.,y+1.,z+1.],
            [x+1.,y+1.,z+0.],
            ],
    // -Y
    Vec3::NEG_Y => 
            [
            [x+0.,y+0.,z+0.],
            [x+1.,y+0.,z+0.],
            [x+1.,y+0.,z+1.],
            [x+0.,y+0.,z+1.],
            ],
    // +Z
    Vec3::Z =>
            [
            [x+1.,y+0.,z+1.],
            [x+1.,y+1.,z+1.],
            [x+0.,y+1.,z+1.],
            [x+0.,y+0.,z+1.],
            ],
    // -Z
    Vec3::NEG_Z =>
            [
            [x+1.,y+0.,z+0.],
            [x+0.,y+0.,z+0.],
            [x+0.,y+1.,z+0.],
            [x+1.,y+1.,z+0.],
            ],
    _ => panic!("Not cardinal dir"),
    }
}
