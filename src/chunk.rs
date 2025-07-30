#![allow(private_interfaces)]
use my_math::vec::*;

use std::mem::MaybeUninit;
use std::time::Instant;
use crate::utils;

use crate::brickmap::{BRICK_SIZE,Voxel,BrickMap};

use fast_noise_lite_rs::{FastNoiseLite, NoiseType};

pub const SEED: u64 = 1111;
pub const CHUNK_SIZE: usize = 1 << 9;

pub type Chunk = crate::entity::Entity;

pub fn gen_brickmap_2d(pos: IVec3) -> BrickMap {
    let mut brick_map = BrickMap::new(ivec3!(CHUNK_SIZE));

    let mut noise = FastNoiseLite::new(SEED as i32);
    noise.set_noise_type(NoiseType::Perlin);
    noise.set_frequency(0.0035);

    let get_height = |x,z| {
        let n = (noise.get_noise_2d(
            (pos.x * CHUNK_SIZE as i32 + x) as f32 ,
            (pos.z * CHUNK_SIZE as i32 + z) as f32 ,
        )+1.)/2. * 50.;
        n.clamp(0.,self::CHUNK_SIZE as f32 )
    };

    for x in 0..CHUNK_SIZE as i32{
        for z in 0..CHUNK_SIZE as i32{
            let max_y = get_height(x,z) ;//* 30. + 40.;
            let mut y = 0.;
            while y  < max_y {
                let mut color = utils::colors::RED;

                if ((x / 8) % 2 == 0) ^ ((z / 8) %2 == 0) ^ ((y as i32 / 8) %2 == 0){
                    color.ch.g = 0b00111111;
                }                 
                brick_map.add_voxel(ivec3!(x,y,z), Voxel{ data:1, color: unsafe{color.col} });
                y += 1.;
            }
        }
    }

    brick_map
}
