use my_math::prelude::*;
use crate::chunk::{self,BrickMap};

pub struct Entity {
    pub brickmap: BrickMap,
    pub brickmap_grid_ssbo: u32,
    pub brickmap_data_ssbo: u32,

    pub pos: Vec3,
    pub orientation: Quaternion,
    pub size: IVec3,
}

pub fn gen_entity() -> Entity {
    let pos = vec3!(15.,313.,12.);
    let orientation = Quaternion::from_axis_angle(Vec3::Y,45.);
    let size = ivec3!(8,16,32);
    let mut brickmap = BrickMap::new(size);    

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
                    brickmap.add_voxel(ivec3!(x,y,z), chunk::Voxel{ data:1, color: 1});
                }
            }
        }
    }
    let ( brickmap_grid_ssbo, brickmap_data_ssbo,) = unsafe { brickmap.gen_ssbos() };

    Entity { 
        brickmap,
        brickmap_grid_ssbo,
        brickmap_data_ssbo,
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
