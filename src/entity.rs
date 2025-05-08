use my_math::prelude::*;

pub struct Entity {
    pos: Vec3,
    orientation: Quaternion,
    size: IVec3,
}

#[allow(unused_variables)]
/// Assuming etities orientation is normalized
fn ray_entity(entity: &Entity, ray_origin: Vec3, ray_dir: Vec3) -> bool {
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
fn ray_aabb(ray_origin: Vec3, ray_dir: Vec3, /*pos: Vec3,*/ size: Vec3) -> Option<f32> {
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
