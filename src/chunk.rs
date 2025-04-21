use my_math::vec::*;

use std::mem::MaybeUninit;
use std::time::Instant;

use crate::octree::Octree;
//use crate::octree::Octree;

use fast_noise_lite_rs::{FastNoiseLite, NoiseType};

pub const SEED: u64 = 1111;
pub const SIZE: usize = 1 << 8;
pub const BRICK_GRID_SIZE:usize = SIZE / 8;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Voxel {
    pub data: u32,
}
pub type Brick = [[[Voxel;8];8];8];
pub type BrickGrid = Box<[[[u32; BRICK_GRID_SIZE]; BRICK_GRID_SIZE]; BRICK_GRID_SIZE]>;
#[repr(C)]
pub struct BrickMap {
    pub brick_grid: BrickGrid,
    pub brick_data: Vec<Brick>,
}
impl BrickMap {
    pub fn new() -> Self {
        Self {
            brick_grid: Box::new([[[u32::MAX;BRICK_GRID_SIZE];BRICK_GRID_SIZE];BRICK_GRID_SIZE]),
            brick_data: Vec::new(),
        }
    }
    pub fn add_voxel(&mut self, pos: IVec3, voxel: Voxel) {
        let grid_coords :IVec3 = pos.div_floor(8);
        let brick_coords:IVec3 = pos.modulo(8);

        let brick = &mut self.brick_grid[grid_coords.x as usize][grid_coords.y as usize][grid_coords.z as usize];

        if *brick == u32::MAX {
            *brick = self.brick_data.len() as u32;
            let mut out = [[[ Voxel{ data: 0 } ;8];8];8];
            out[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
            self.brick_data.push(out);
        } else {
            let data = &mut self.brick_data[*brick as usize];
            data[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
        }
    }
}


pub fn gen_brickmap() -> BrickMap {
    let start = Instant::now();
    let mut brick_map = BrickMap::new();

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

    for x in 0..SIZE as i32{
        for y in 0..SIZE as i32{
            for z in 0..SIZE as i32{
                if has_voxel(x,y,z) {
                    brick_map.add_voxel(ivec3!(x,y,z),Voxel{data:1});
                }
            }
        }
    }
    println!("time (brick map): {:?}",start.elapsed());
    brick_map
}

pub type ChunkData = [[[i32; SIZE]; SIZE]; SIZE];

pub fn gen_chunk_octree_2d() -> Octree {
    let start = Instant::now();
    let mut octree = Octree::new(SIZE as u32,ivec3!(0,0,0));

    let mut noise = FastNoiseLite::new(SEED as i32);
    noise.set_noise_type(NoiseType::Perlin);
    noise.set_frequency(0.015);

    let get_height = |x,z| {
        let n = noise.get_noise_2d(
            x as f32 ,
            z as f32 ,
        );
        n
    };

    for x in 0..SIZE as i32{
        for z in 0..SIZE as i32{
            let max_y = get_height(x,z) * 10. + 10.;
            let mut y = 0;
            while (y as f32) < max_y {
                octree.add_block(ivec3!(x,y,z));
                y += 1;
            }
        }
    }
    octree.nodes.shrink_to_fit();
    octree.nodes.reserve_exact(1000);
    println!("time (octree): {:?}",start.elapsed());
    //panic!();
    return octree;
}
pub fn gen_chunk_octree() -> Octree {
    let start = Instant::now();
    let mut octree = Octree::new(SIZE as u32,ivec3!(0,0,0));

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

    for x in 0..SIZE as i32{
        for y in 0..SIZE as i32{
            for z in 0..SIZE as i32{
                if has_voxel(x,y,z) {
                    octree.add_block(ivec3!(x,y,z));
                }
            }
        }
    }
    println!("time (octree): {:?}",start.elapsed());
    return octree;

    //fill_node(&mut octree.head);
//
    //fn fill_node(node : &mut OctreeNode) {
        //if node.children.is_none() {
             //
        //}
    //}
}

pub fn gen_chunk_data_2d() -> Box<ChunkData> {
    let start = Instant::now();
    let mut noise = FastNoiseLite::new(SEED as i32);
    noise.set_noise_type(NoiseType::Perlin);
    noise.set_frequency(0.015);

    let get_height = |x,z| {
        let n = noise.get_noise_2d(
            x as f32 ,
            z as f32 ,
        );
        n
    };

    let mut count = 0;
    let mut chunk_data: Box<[[[MaybeUninit<i32>; SIZE]; SIZE]; SIZE]> = Box::new([[[MaybeUninit::<i32>::uninit();SIZE];SIZE];SIZE]);

    for x in 0..SIZE as i32{
        for z in 0..SIZE as i32{
            let max_y = get_height(x,z) * 10. + 10.;
            let mut y = 0;
            while (y as f32) < max_y {
                chunk_data[x as usize][y as usize][z as usize] = MaybeUninit::new(1);
                count += 1;
                y += 1;
            }
        }
    }

    println!("time (array): {:?}",start.elapsed());
    println!("cubes: {}",count);
    return unsafe { Box::from_raw(Box::into_raw(chunk_data) as *mut ChunkData) };
}
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
        return n >= 0.;
    };

    let mut count = 0;
    let mut chunk_data: Box<[[[MaybeUninit<i32>; SIZE]; SIZE]; SIZE]> = Box::new([[[MaybeUninit::<i32>::uninit();SIZE];SIZE];SIZE]);
    for x in 0..SIZE {
        for y in 0..SIZE {
            for z in 0..SIZE {
                let voxel = has_voxel(x,y,z);
                chunk_data[x as usize][y as usize][z as usize] = MaybeUninit::new(voxel as i32);
                if voxel { count += 1 };
            }
        }
    }
    println!("time (array): {:?}",start.elapsed());
    println!("cubes: {}",count);
    return unsafe { Box::from_raw(Box::into_raw(chunk_data) as *mut ChunkData) };
}
/*
pub fn gen_mesh(chunk_data: &Box<ChunkData>) -> Mesh<Vertex> {
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
*/
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
