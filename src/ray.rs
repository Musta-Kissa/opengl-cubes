use my_math::prelude::*;
use crate::chunk::SIZE;
use crate::chunk::ChunkData;
use crate::octree::*;

fn next_node(curr_quad: i32, txm: f32, tym: f32,tzm: f32) -> i32 {
    const EXIT: i32 = 8;
    let mut exit_idx = 2; // XY
    if (txm < tym) {
		if(txm < tzm){ exit_idx = 0}  // YZ plane
	}else{
		if(tym < tzm){ exit_idx = 1} // XZ plane
	}

    let exit_lookup = [
        [  4,   2,    1  ],
        [  5,   3,   EXIT],
        [  6,  EXIT,  3  ],
        [  7,  EXIT, EXIT],
        [EXIT,  6,    5  ],
        [EXIT,  7,   EXIT],
        [EXIT, EXIT,  7  ],
        [EXIT, EXIT, EXIT],
    ];

    return exit_lookup[curr_quad as usize][exit_idx];
}

fn first_node (tx0: f32, ty0: f32, tz0: f32,txm: f32, tym: f32,tzm:f32) -> i32 {
    let mut out = 0;
    if tx0 > ty0 && tx0 > tz0 {
        // YZ plane
        if tym < tx0 { out |= 2; }
        if tzm < tx0 { out |= 1; }
    } else if ty0 > tz0 {
        // XZ plane
        if txm < ty0 { out |= 4; }
        if tzm < ty0 { out |= 1; }
    } else {
        // XY plane
        if txm < tz0 { out |= 4; }
        if tym < tz0 { out |= 2; }
    }
    return out;
}

pub fn ray_octree<'a>(start: Vec3, dir: Vec3, chunk_data: &'a Octree) -> Option<(&'a OctreeNode,f32)> {
    let mut start = start;

    let node = &chunk_data.head;

    let node_pos = node.position ;

    let mut mask:u8 = 0;
    if dir.x < 0. {
        start.x = 2. * node_pos.x as f32 + node.size as f32 - start.x;
        mask |= 4;
    }
    if dir.y < 0. {
        start.y = 2. * node_pos.y as f32 + node.size as f32 - start.y;
        mask |= 2;
    }
    if dir.z < 0. {
        start.z = 2. * node_pos.z as f32 + node.size as f32 - start.z;
        mask |= 1;
    }
    //println!("mask {:03b}",mask);

    let tx0 = (node_pos.x as f32 - start.x) / dir.x.abs();
    let ty0 = (node_pos.y as f32 - start.y) / dir.y.abs();
    let tz0 = (node_pos.z as f32 - start.z) / dir.z.abs();

    let tx1 = (node_pos.x as f32 + node.size as f32 - start.x) / dir.x.abs();
    let ty1 = (node_pos.y as f32 + node.size as f32 - start.y) / dir.y.abs();
    let tz1 = (node_pos.z as f32 + node.size as f32 - start.z) / dir.z.abs();

    let t_min = tx0.max(ty0).max(tz0);
    let t_max = tx1.min(ty1).min(tz1);

    let intersects: bool = t_min < t_max ;

    if !intersects {
        println!("no intersection");
        return None;
    }

    return proc_subtree(start,dir,mask,node,tx0,ty0,tz0,tx1,ty1,tz1);

    fn proc_subtree(start: Vec3, dir: Vec3, mask: u8,node: &OctreeNode,
                    tx0:f32,ty0:f32,tz0:f32,tx1:f32,ty1:f32,tz1:f32) -> Option<(&OctreeNode,f32)> {
        if node.children.is_none() {
            if node.is_full && ( tx1 >= 0. && ty1 >= 0. && tz1 >= 0.) {
                if tx0 < 0. && ty0 < 0. && tz0 < 0.{
                    return Some((node,0.));
                }else {
                    let t_min = tx0.max(ty0).max(tz0);
                    return Some((node,t_min));
                }
            } else {
                return None;
            }
        }

        let txm = (tx0 + tx1) /2.;
        let tym = (ty0 + ty1) /2.;
        let tzm = (tz0 + tz1) /2.;

        let mut curr_node = first_node(tx0,ty0,tz0,txm,tym,tzm);

        let children = &node.children.as_ref().unwrap();
        //println!("hit index {curr_node}");

        while curr_node < 8 {
            let child = &children[curr_node as usize ^ mask as usize];
            match curr_node {
                0 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, tx0, ty0, tz0, txm, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tzm);
                },
                1 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, 
                                                        tx0,ty0,tzm,txm,tym,tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tz1);
                },
                2 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, tx0, tym, tz0, txm, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,ty1,tzm);
                },
                3 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, tx0, tym, tzm, txm, ty1, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,txm, ty1, tz1,);
                },
                4 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, ty0, tz0, tx1, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,tx1,  tym,  tzm);
                },
                5 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, ty0, tzm, tx1, tym, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  tym,  tz1);
                },
                6 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, tym, tz0, tx1, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  ty1,  tzm );
                },
                7 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, tym, tzm, tx1, ty1, tz1 ) {
                        return Some((hit,t));
                    }
                    curr_node = 8;
                },
                _ => panic!(),
            }
        }
        return None;
    }
}
pub fn ray_octree_exact<'a>(start: Vec3, dir: Vec3, chunk_data: &'a Octree) -> Option<(&'a OctreeNode,f32)> {
    let mut start = start;

    let node = &chunk_data.head;

    let node_pos = node.position ;

    let mut mask:u8 = 0;
    if dir.x < 0. {
        start.x = 2. * node_pos.x as f32 + node.size as f32 - start.x;
        mask |= 4;
    }
    if dir.y < 0. {
        start.y = 2. * node_pos.y as f32 + node.size as f32 - start.y;
        mask |= 2;
    }
    if dir.z < 0. {
        start.z = 2. * node_pos.z as f32 + node.size as f32 - start.z;
        mask |= 1;
    }
    //println!("mask {:03b}",mask);

    let tx0 = (node_pos.x as f32 - start.x) / dir.x.abs();
    let ty0 = (node_pos.y as f32 - start.y) / dir.y.abs();
    let tz0 = (node_pos.z as f32 - start.z) / dir.z.abs();

    let tx1 = (node_pos.x as f32 + node.size as f32 - start.x) / dir.x.abs();
    let ty1 = (node_pos.y as f32 + node.size as f32 - start.y) / dir.y.abs();
    let tz1 = (node_pos.z as f32 + node.size as f32 - start.z) / dir.z.abs();

    let t_min = tx0.max(ty0).max(tz0);
    let t_max = tx1.min(ty1).min(tz1);

    let intersects: bool = t_min < t_max ;

    if !intersects {
        println!("no intersection");
        return None;
    }

    return proc_subtree(start,dir,mask,node,tx0,ty0,tz0,tx1,ty1,tz1);

    fn proc_subtree(start: Vec3, dir: Vec3, mask: u8,node: &OctreeNode,
                    tx0:f32,ty0:f32,tz0:f32,tx1:f32,ty1:f32,tz1:f32) -> Option<(&OctreeNode,f32)> {
        if node.children.is_none() {
            if node.is_full && ( tx1 >= 0. && ty1 >= 0. && tz1 >= 0.) {
                if tx0 < 0. && ty0 < 0. && tz0 < 0.{
                    return Some((node,0.));
                }else {
                    let t_min = tx0.max(ty0).max(tz0);
                    return Some((node,t_min));
                }
            } else {
                return None;
            }
        }

        let txm = (tx0 + tx1) /2.;
        let tym = (ty0 + ty1) /2.;
        let tzm = (tz0 + tz1) /2.;

        let mut curr_node = first_node(tx0,ty0,tz0,txm,tym,tzm);

        let children = &node.children.as_ref().unwrap();
        //println!("hit index {curr_node}");

        while curr_node < 8 {
            let child = &children[curr_node as usize ^ mask as usize];
            match curr_node {
                0 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, tx0, ty0, tz0, txm, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tzm);
                },
                1 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, 
                                                        tx0,ty0,tzm,txm,tym,tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tz1);
                },
                2 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, tx0, tym, tz0, txm, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,ty1,tzm);
                },
                3 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, tx0, tym, tzm, txm, ty1, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,txm, ty1, tz1,);
                },
                4 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, ty0, tz0, tx1, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,tx1,  tym,  tzm);
                },
                5 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, ty0, tzm, tx1, tym, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  tym,  tz1);
                },
                6 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, tym, tz0, tx1, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  ty1,  tzm );
                },
                7 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, child, txm, tym, tzm, tx1, ty1, tz1 ) {
                        return Some((hit,t));
                    }
                    curr_node = 8;
                },
                _ => panic!(),
            }
        }
        return None;
    }
}



use std::f32;

pub fn dda_3d_octree(start: Vec3, dir: Vec3, max_distance: f32,octree:&Octree) -> Option<(IVec3,Vec3)>{
    let mut voxel = IVec3::new(
                            start.x.floor() as i32, 
                            start.y.floor() as i32, 
                            start.z.floor() as i32 
                        );

    let step_dir = IVec3::new(
                            dir.x.signum() as i32,
                            dir.y.signum() as i32,
                            dir.z.signum() as i32
                        );

    let mut t_delta = Vec3::new( 
                            1. / dir.x.abs(), 
                            1. / dir.y.abs(), 
                            1. / dir.z.abs() 
                        );

    fn frac0(x: f32) -> f32 {
        x - x.floor()
    }
    fn frac1(x: f32) -> f32 {
        1.0 - x + x.floor()
    }
    let mut t_max_x = if dir.x > 0. {
        t_delta.x * frac1(start.x)
    } else {
        t_delta.x * frac0(start.x)
    };

    let mut t_max_y = if dir.y > 0. {
        t_delta.y * frac1(start.y)
    } else {
        t_delta.y * frac0(start.y)
    };

    let mut t_max_z = if dir.z > 0. {
        t_delta.z * frac1(start.z)
    } else {
        t_delta.z * frac0(start.z)
    };

    let mut traveled_distance = 0.0;
    while traveled_distance < max_distance {

        if octree.is_solid_at(voxel) {
            return Some((voxel,start + dir * traveled_distance));
        }

        // Possibly faster using
        //let should_increment_x = (t_max_x <= t_max_y && t_max_x <= t_max_z) as i32;
        //let should_increment_y = (t_max_y <= t_max_x && t_max_y <= t_max_z) as i32;
        //let should_increment_z = (t_max_z <= t_max_x && t_max_z <= t_max_y) as i32;

        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                voxel.x += step_dir.x;
                traveled_distance = t_max_x;
                t_max_x += t_delta.x;
            } else {
                voxel.z += step_dir.z;
                traveled_distance = t_max_z;
                t_max_z += t_delta.z;
            }
        } else {
            if t_max_y < t_max_z {
                voxel.y += step_dir.y;
                traveled_distance = t_max_y;
                t_max_y += t_delta.y;
            } else {
                voxel.z += step_dir.z;
                traveled_distance = t_max_z;
                t_max_z += t_delta.z;
            }
        }
    }
    None
}
pub fn dda_3d(start: Vec3, dir: Vec3, max_distance: f32,chunk_data:&Box<ChunkData>) -> Option<(IVec3,Vec3)>{
    let mut voxel = IVec3::new(
                            start.x.floor() as i32, 
                            start.y.floor() as i32, 
                            start.z.floor() as i32 
                        );

    let step_dir = IVec3::new(
                            dir.x.signum() as i32,
                            dir.y.signum() as i32,
                            dir.z.signum() as i32
                        );

    let mut t_delta = Vec3::new( 
                            1. / dir.x.abs(), 
                            1. / dir.y.abs(), 
                            1. / dir.z.abs() 
                        );

    fn frac0(x: f32) -> f32 {
        x - x.floor()
    }
    fn frac1(x: f32) -> f32 {
        1.0 - x + x.floor()
    }
    let mut t_max_x = if dir.x > 0. {
        t_delta.x * frac1(start.x)
    } else {
        t_delta.x * frac0(start.x)
    };

    let mut t_max_y = if dir.y > 0. {
        t_delta.y * frac1(start.y)
    } else {
        t_delta.y * frac0(start.y)
    };

    let mut t_max_z = if dir.z > 0. {
        t_delta.z * frac1(start.z)
    } else {
        t_delta.z * frac0(start.z)
    };

    let mut traveled_distance = 0.0;
    while traveled_distance < max_distance {

        if !(voxel.x < 0 || voxel.x >= SIZE as i32 
            || voxel.y < 0 || voxel.y >= SIZE as i32 
            || voxel.z < 0 || voxel.z >= SIZE as i32) {
            if chunk_data[voxel.x as usize][voxel.y as usize][voxel.z as usize] {
                return Some((voxel,start + dir * traveled_distance));
            }
        }

        // Possibly faster using
        //let should_increment_x = (t_max_x <= t_max_y && t_max_x <= t_max_z) as i32;
        //let should_increment_y = (t_max_y <= t_max_x && t_max_y <= t_max_z) as i32;
        //let should_increment_z = (t_max_z <= t_max_x && t_max_z <= t_max_y) as i32;

        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                voxel.x += step_dir.x;
                traveled_distance = t_max_x;
                t_max_x += t_delta.x;
            } else {
                voxel.z += step_dir.z;
                traveled_distance = t_max_z;
                t_max_z += t_delta.z;
            }
        } else {
            if t_max_y < t_max_z {
                voxel.y += step_dir.y;
                traveled_distance = t_max_y;
                t_max_y += t_delta.y;
            } else {
                voxel.z += step_dir.z;
                traveled_distance = t_max_z;
                t_max_z += t_delta.z;
            }
        }
    }
    None
}
