#![allow(private_interfaces)]
use my_math::vec::*;

use std::mem::MaybeUninit;
use std::time::Instant;
use crate::utils;

use crate::octree::Octree;

use fast_noise_lite_rs::{FastNoiseLite, NoiseType};

pub const SEED: u64 = 1111;
pub const SIZE: usize = 1 << 9;
pub const BRICK_SIZE: usize = 8;
pub const BRICK_GRID_SIZE:usize = SIZE / BRICK_SIZE;

pub struct Chunk {
    pub brickmap: BrickMap,
    pub brickmap_grid_ssbo: u32,
    pub brickmap_data_ssbo: u32,
    pub pos: IVec3,
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Voxel {
    pub data: u32,
    pub color: u32,
}
pub type Brick = [[[Voxel;8];8];8];
pub type BrickGrid = [[[u32; BRICK_GRID_SIZE]; BRICK_GRID_SIZE]; BRICK_GRID_SIZE];
#[repr(C)]
pub struct BrickMap {
    pub grid: Box<BrickGrid>,
    pub data: Vec<Brick>,
}
impl BrickMap {
    pub fn new() -> Self {
        Self {
            grid: Box::new([[[u32::MAX;BRICK_GRID_SIZE];BRICK_GRID_SIZE];BRICK_GRID_SIZE]),
            data: Vec::new(),
        }
    }
    pub fn add_voxel(&mut self, pos: IVec3, voxel: Voxel) {
        let grid_coords :IVec3 = pos.div_floor(8);
        let brick_coords:IVec3 = pos.modulo(8);

        let brick = &mut self.grid[grid_coords.x as usize][grid_coords.y as usize][grid_coords.z as usize];

        if *brick == u32::MAX {
            *brick = self.data.len() as u32;
            let mut out = [[[ Voxel{ data: 0 , color: utils::simple_rng_u32()} ;8];8];8];
            out[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
            self.data.push(out);
        } else {
            let data = &mut self.data[*brick as usize];
            data[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
        }
    }
    pub unsafe fn gen_ssbos(&self) -> (u32,u32) {
        use std::mem;
        let chunk_brickmap = &self;

        let mut brick_grid_ssbo = 0;
        let mut brick_data_ssbo = 0;

        gl::GenBuffers(1, &mut brick_grid_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_grid_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            (mem::size_of::<self::BrickGrid>()) as isize,
            chunk_brickmap.grid.as_ptr() as *const _,
            gl::DYNAMIC_DRAW,
        );
        
        // Allocate buffer for Brick data, but don't fill it yet
        let total_size = mem::size_of::<self::Brick>() * chunk_brickmap.data.len();
        gl::GenBuffers(1, &mut brick_data_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_data_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            total_size as isize,
            std::ptr::null(), // no initial data
            gl::DYNAMIC_DRAW,
        );

        // Upload data in chunks
        let chunk_size = 1024^2;
        let brick_size = mem::size_of::<self::Brick>();
        let mut offset = 0;
        for chunk in chunk_brickmap.data.chunks(chunk_size) {
            let byte_size = brick_size * chunk.len();
            gl::BufferSubData(
                gl::SHADER_STORAGE_BUFFER,
                offset as isize,
                byte_size as isize,
                chunk.as_ptr() as *const _,
            );
            unsafe { gl::Finish() };
            std::thread::sleep(std::time::Duration::from_micros(150));
            offset += byte_size;
        }

        gl::MemoryBarrier(gl::SHADER_STORAGE_BARRIER_BIT);
        
        (brick_grid_ssbo, brick_data_ssbo)
    }
}


pub fn gen_brickmap_2d(pos: IVec3) -> BrickMap {
    let mut brick_map = BrickMap::new();

    let mut noise = FastNoiseLite::new(SEED as i32);
    noise.set_noise_type(NoiseType::Perlin);
    noise.set_frequency(0.0035);

    let get_height = |x,z| {
        //let mut n = (noise_3.get_noise_2d(
            //((pos.x * self::SIZE as i32+ x ) as f32) / 200.,
            //((pos.z * self::SIZE as i32+ z ) as f32) / 200.,
        //) + 1.)
            //* 16.;
//
        //n += (noise_3.get_noise_2d(
            //((pos.x * self::SIZE as i32 + x ) as f32) / 1000.,
            //((pos.z * self::SIZE as i32 + z ) as f32) / 1000.,
        //) + 1.)
            //* 16.
            //* 4.;
        //n -= 32.;
        let mut n = (noise.get_noise_2d(
            (pos.x * SIZE as i32 + x) as f32 ,
            (pos.z * SIZE as i32 + z) as f32 ,
        )+1.)/2. * 50.;
        //n += noise_2.get_noise_2d(
            //(pos.x * SIZE as i32 + x) as f32 ,
            //(pos.z * SIZE as i32 + z) as f32 ,
        //) * 2.;
        n.clamp(0.,self::SIZE as f32 )
    };

    for x in 0..SIZE as i32{
        for z in 0..SIZE as i32{
            let max_y = get_height(x,z) ;//* 30. + 40.;
            let mut y = 0.;
            while y  < max_y {
                let mut color = RED;
                //let ratio = y as f64 /50.  as f64 ;
                //let mut color = blend_color(RED,BLUE, ratio);
                unsafe {
                    if ((x / 8) % 2 == 0) ^ ((z / 8) %2 == 0) ^ ((y as i32 / 8) %2 == 0){
                        color.ch.g = 0b00111111;
                    } else {
                    }
                }
                //brick_map.add_voxel(ivec3!(x,y,z),Voxel{data:1, color: utils::simple_rng_u32()});
                //let color = unsafe { std::mem::transmute::<f32,u32>(
                    //noise.get_noise_3d(
                        ////(pos.x*SIZE as i32 + x/8) as f32,
                        ////(pos.y*SIZE as i32 + y/8) as f32,
                        ////(pos.z*SIZE as i32 + z/8) as f32)) + 10
                        //(x/8) as f32,
                        //(y/8) as f32,
                        //(z/8) as f32)) + (1 << 16) + (1 << 8) + 1
                //};
                //let color = unsafe { std::mem::transmute::<f32,u32>(noise.get_noise_2d(y,y)) };
                brick_map.add_voxel(ivec3!(x,y,z), Voxel{ data:1, color: unsafe{color.col} });
                y += 1.;
            }
        }
    }

    //println!("time (brick map): {:?}",start.elapsed());
    brick_map
}
pub const RED: Color = Color { col: ((1u32 << 9) - 1) << 16 };
pub const BLUE: Color = Color { col: (1u32 << 9) - 1 };
// The order is reversed in memory
#[derive(Clone, Copy)]
struct ColorChanels {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}
#[derive(Clone, Copy)]
union Color {
    col: u32,
    ch: ColorChanels,
}
impl std::ops::Mul<f64> for Color {
    type Output = Color;
    fn mul(self, rhs: f64) -> Self::Output {
        unsafe {
            let a = self.ch.a;
            let r = (self.ch.r as f64 * rhs).floor() as u8;
            let g = (self.ch.g as f64 * rhs).floor() as u8;
            let b = (self.ch.b as f64 * rhs).floor() as u8;
            Color {
                ch: ColorChanels { a, r, g, b },
            }
        }
    }
}

fn blend_color(c1: Color, c2: Color, ratio: f64) -> Color {
    unsafe {
        Color {
            ch: ColorChanels {
                a: c1.ch.a,
                r: (c1.ch.r as f64 * ratio + c2.ch.r as f64 * (1. - ratio)).round() as u8,
                g: (c1.ch.g as f64 * ratio + c2.ch.g as f64 * (1. - ratio)).round() as u8,
                b: (c1.ch.b as f64 * ratio + c2.ch.b as f64 * (1. - ratio)).round() as u8,
            },
        }
    }
}


pub fn gen_brickmap(pos: IVec3) -> BrickMap {
    let start = Instant::now();
    let mut brick_map = BrickMap::new();
    let mut voxel_count = 0;

    let mut noise = FastNoiseLite::new(SEED as i32);
    noise.set_noise_type(NoiseType::Perlin);
    noise.set_frequency(0.035);

    let has_voxel = |x,y,z| {
        let n = noise.get_noise_3d(
            (pos.x + x) as f32 ,
            (pos.y + y) as f32 ,
            (pos.z + z) as f32 ,
        );
        return n >= 0.
    };

    for x in 0..SIZE as i32{
        for y in 0..SIZE as i32{
            for z in 0..SIZE as i32{
                if has_voxel(x,y,z) {
                    brick_map.add_voxel(ivec3!(x,y,z),Voxel{data:1, color: utils::simple_rng_u32()});
                    voxel_count += 1;
                }
            }
        }
    }
    println!("time (brick map): {:?}",start.elapsed());
    println!("{} voxels",voxel_count);
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
