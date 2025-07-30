#![allow(private_interfaces)]
use my_math::vec::*;

use std::mem::MaybeUninit;
use std::time::Instant;
use crate::utils;

use crate::brickmap::{BRICK_SIZE,Voxel,BrickMap};

use fast_noise_lite_rs::{FastNoiseLite, NoiseType};

pub const SEED: u64 = 1111;
pub const CHUNK_SIZE: usize = 1 << 9;
//pub const BRICK_GRID_SIZE:usize = CHUNK_SIZE / BRICK_SIZE;

pub type Chunk = crate::entity::Entity;
/*pub struct Chunk {
    pub brickmap: BrickMap,
    pub brickmap_grid_ssbo: u32,
    pub brickmap_data_ssbo: u32,
    pub pos: IVec3,
}*/

pub fn gen_brickmap_2d(pos: IVec3) -> BrickMap {
    let mut brick_map = BrickMap::new(ivec3!(CHUNK_SIZE));

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
        let n = (noise.get_noise_2d(
            (pos.x * CHUNK_SIZE as i32 + x) as f32 ,
            (pos.z * CHUNK_SIZE as i32 + z) as f32 ,
        )+1.)/2. * 50.;
        //n += noise_2.get_noise_2d(
            //(pos.x * SIZE as i32 + x) as f32 ,
            //(pos.z * SIZE as i32 + z) as f32 ,
        //) * 2.;
        n.clamp(0.,self::CHUNK_SIZE as f32 )
    };

    for x in 0..CHUNK_SIZE as i32{
        for z in 0..CHUNK_SIZE as i32{
            let max_y = get_height(x,z) ;//* 30. + 40.;
            let mut y = 0.;
            while y  < max_y {
                let mut color = utils::colors::RED;
                //let ratio = y as f64 /50.  as f64 ;
                //let mut color = blend_color(RED,BLUE, ratio);

                if ((x / 8) % 2 == 0) ^ ((z / 8) %2 == 0) ^ ((y as i32 / 8) %2 == 0){
                    color.ch.g = 0b00111111;
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
/*
pub fn gen_brickmap(pos: IVec3) -> BrickMap {
    let start = Instant::now();
    let mut brick_map = BrickMap::new(ivec3!(BRICK_GRID_SIZE));
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

    for x in 0..CHUNK_SIZE as i32{
        for y in 0..CHUNK_SIZE as i32{
            for z in 0..CHUNK_SIZE as i32{
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

*/
