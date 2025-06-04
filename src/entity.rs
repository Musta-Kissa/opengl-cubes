use my_math::prelude::*;
use crate::chunk::{Brick,Voxel,BRICK_SIZE};
use crate::utils;
use crate::allocator::BrickVec;

#[repr(C)]
pub struct Entity {
    pub brickmap_grid: Vec<u32>,
    pub brickmap_grid_ssbo: u32,

    pub brickmap_data: BrickVec,

    pub pos: Vec3,
    pub orientation: Quaternion,
    pub size: IVec3,
}

pub fn brickmap_brick_index_at<'a>(brickmap_grid: &'a mut Vec<u32>, brickmap_size: IVec3, idx: IVec3) -> &'a mut u32 {
    let index = idx.x * brickmap_size.y/8 * brickmap_size.z/8 + 
                idx.y * brickmap_size.z/8 + 
                idx.z;
    return &mut brickmap_grid[index as usize];
}

pub unsafe fn brickmap_gen_ssbos(
    brickmap_grid: &Vec<u32>, 
    //brickmap_data: &Vec<Brick>, 
) -> (u32,u32) {
    use std::mem;

    let mut brick_grid_ssbo = 0;
    let mut brick_data_ssbo = 0;

    gl::GenBuffers(1, &mut brick_grid_ssbo);
    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_grid_ssbo);
    gl::BufferData(
        gl::SHADER_STORAGE_BUFFER,
        (brickmap_grid.len() * mem::size_of::<u32>()) as isize,
        brickmap_grid.as_ptr() as *const _,
        gl::DYNAMIC_DRAW,
    );

    /*
    
    // Allocate buffer for Brick data, but don't fill it yet
    gl::GenBuffers(1, &mut brick_data_ssbo);
    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_data_ssbo);
    gl::BufferData(
        gl::SHADER_STORAGE_BUFFER,
        (brickmap_data.len() * mem::size_of::<Brick>()) as isize,
        std::ptr::null(), // no initial data
        gl::DYNAMIC_DRAW,
    );

    // Upload data in chunks
    let chunk_size = 1024^2;
    let brick_size = mem::size_of::<Brick>();
    let mut offset = 0;
    for chunk in (brickmap_data).chunks(chunk_size) {
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
    */

    gl::MemoryBarrier(gl::SHADER_STORAGE_BARRIER_BIT);
    
    (brick_grid_ssbo, brick_data_ssbo)
}

pub fn brickmap_add_voxel(
    brickmap_grid: &mut Vec<u32>, 
    brickmap_data: &mut Vec<Brick>, 
    brickmap_size: IVec3, 
    voxel_pos: IVec3, 
    voxel: Voxel
) {
    let grid_coords :IVec3 = voxel_pos / 8;
    let brick_coords:IVec3 = voxel_pos % 8;

    let brickmap_data_len = brickmap_data.len();
    let brick = brickmap_brick_index_at(brickmap_grid,brickmap_size,grid_coords);

    if *brick == u32::MAX {
        *brick = brickmap_data_len as u32;
        let mut out = [[[ Voxel{ data: 0 , color: utils::simple_rng_u32()} ;BRICK_SIZE];BRICK_SIZE];BRICK_SIZE];
        out[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
        brickmap_data.push(out);
    } else {
        let brick = *brick;
        brickmap_data[brick as usize]
            [brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
    }
}

pub fn gen_entity() -> Entity {
    let pos = vec3!(15.,313.,12.);
    let orientation = Quaternion::from_axis_angle(Vec3::Y,45.);
    let size = ivec3!(8,16,32);
    let len = (size.x/8 * size.y/8 * size.z/8) as usize;

    let mut brickmap_grid:Vec<u32>   = vec![u32::MAX;len];
    let mut brickmap_data: Vec<Brick> = Vec::new();

    let center = size.as_vec3() / 2.0; // Center of the sphere

    let is_in_ellipsoid = |x:i32, y:i32, z:i32| {
        let normalized_x = (x as f32 - center.x) / center.x;
        let normalized_y = (y as f32 - center.y) / center.y;
        let normalized_z = (z as f32 - center.z) / center.z;

        return normalized_x.powi(2) + normalized_y.powi(2) + normalized_z.powi(2) < 1.0;
    };

    for x in 0..size.x {
        for y in 0..size.y {
            for z in 0..size.z {
                if is_in_ellipsoid(x,y,z) {
                    brickmap_add_voxel(&mut brickmap_grid, &mut brickmap_data, size, ivec3!(x,y,z), Voxel{ data:1, color: 1});
                }
            }
        }
    }
    let brickmap_data = BrickVec::from_vec(brickmap_data);
    unsafe { brickmap_data.send() };

    let ( brickmap_grid_ssbo, _brickmap_data_ssbo) = unsafe { brickmap_gen_ssbos(&brickmap_grid) };

    Entity { 
        brickmap_grid,
        brickmap_data,
        brickmap_grid_ssbo,
        pos,
        orientation,
        size,
    }
}

pub fn ray_to_local(entity: &Entity, ray_origin:Vec3,ray_dir:Vec3) -> (Vec3,Vec3) {
    let half_size = (entity.size/2).as_vec3();
    let entity_middle = entity.pos + half_size;
    let inv_orientation = entity.orientation.conjugate();
    // transform the ray_origin to local coordinates 
    let relative_pos = ray_origin - entity_middle;

    // rotate around the middle of the entity in local coordinates 
    // and move the ray so the entities neg corner is at 0,0,0
    let ray_origin_local    = rot_vec_by_quat(relative_pos,&inv_orientation) + half_size;
    let ray_dir_local       = rot_vec_by_quat(ray_dir,&inv_orientation);
    (ray_origin_local,ray_dir_local)
}

#[allow(unused_variables)]
/// Assuming etities orientation is normalized
pub fn ray_entity(entity: &Entity, ray_origin: Vec3, ray_dir: Vec3) -> bool {
    let half_size = (entity.size/2).as_vec3();
    let entity_middle = entity.pos + half_size;
    let inv_orientation = entity.orientation.conjugate();
    // transform the ray_origin to local coordinates 
    let relative_pos = ray_origin - entity_middle;

    // rotate around the middle of the entity in local coordinates 
    // and move the ray so the entities neg corner is at 0,0,0
    let ray_origin_local    = rot_vec_by_quat(relative_pos,&inv_orientation) + half_size;
    let ray_dir_local       = rot_vec_by_quat(ray_dir,&inv_orientation);
    
    if ray_aabb(ray_origin_local,ray_origin_local,entity.size.as_vec3()).is_some() {
        true
    } else {
        false
    }
}

/// Assuming the aabb neg corner is at 0,0,0
pub fn ray_aabb(ray_origin: Vec3, ray_dir: Vec3, /*pos: Vec3,*/ size: Vec3) -> Option<f32> {
    //let min_corner = pos;
    //let max_corner = pos ze;

    let inv_dir = 1.0 / ray_dir;

    //let t0 = (min_corner - ray_origin) * inv_dir;
    //let t1 = (max_corner - ray_origin) * inv_dir; 
    let t0 = (Vec3::ZERO        - ray_origin) * inv_dir;
    let t1 = (Vec3::ZERO + size - ray_origin) * inv_dir; 

    let t_min = vec3!(
        t0.x.min(t1.x),
        t0.y.min(t1.y),
        t0.z.min(t1.z)
    );
    let t_max = vec3!(
        t0.x.max(t1.x),
        t0.y.max(t1.y),
        t0.z.max(t1.z)
    );


    let t_enter = (t_min.x).max(t_min.y).max(t_min.z);
    let t_exit  = (t_max.x).min(t_max.y).min(t_max.z);

    if t_exit >= t_enter && t_exit > 0.0 {
        return Some(t_enter)
    } else {
        return None;
    }
}
