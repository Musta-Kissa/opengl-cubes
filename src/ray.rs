use my_math::prelude::*;
use crate::chunk::SIZE;
use crate::chunk::ChunkData;
use crate::octree::*;

fn next_node(curr_quad: i32, txm: f32, tym: f32,tzm: f32) -> i32 {
    const EXIT: i32 = 8;
    let mut exit_idx = 2; // XY
    if txm < tym {
		if txm < tzm { exit_idx = 0 }  // YZ plane
	}else{
		if tym < tzm { exit_idx = 1 } // XZ plane
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

#[derive(Debug,Copy,Clone)]
pub enum Dir {
    X,
    NegX,
    Y,
    NegY,
    Z,
    NegZ,
}
impl std::ops::Neg for Dir {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Dir::X => Dir::NegX,
            Dir::NegX => Dir::X,
            Dir::Y => Dir::NegY,
            Dir::NegY => Dir::Y,
            Dir::Z => Dir::NegZ,
            Dir::NegZ => Dir::Z,
        }
    }
}
impl Into<Vec3> for Dir {
    fn into(self) -> Vec3 {
        match self {
            Dir::X => Vec3::X,
            Dir::NegX => Vec3::NEG_X,
            Dir::Y => Vec3::Y,
            Dir::NegY => Vec3::NEG_Y,
            Dir::Z => Vec3::Z,
            Dir::NegZ => Vec3::NEG_Z,
        }
    }
}
impl Into<IVec3> for Dir {
    fn into(self) -> IVec3 {
        match self {
            Dir::X => IVec3::X,
            Dir::NegX => IVec3::NEG_X,
            Dir::Y => IVec3::Y,
            Dir::NegY => IVec3::NEG_Y,
            Dir::Z => IVec3::Z,
            Dir::NegZ => IVec3::NEG_Z,
        }
    }
}

pub fn hit_direction(hit: Vec3, dir: Vec3) -> Dir {
    let x = (hit.x - hit.x.round()).abs();
    let y = (hit.y - hit.y.round()).abs();
    let z = (hit.z - hit.z.round()).abs();

    if x < y && x < z {
        if dir.x < 0. {
            return Dir::NegX;
        } else {
            return Dir::X;
        }
    } else if y < z {
        if dir.y < 0. {
            return Dir::NegY;
        } else {
            return Dir::Y;
        }
    } else {
        if dir.z < 0. {
            return Dir::NegZ;
        } else {
            return Dir::Z;
        }
    }
}

pub fn ray_octree_dir<'a>(start: Vec3, dir: Vec3, chunk_data: &'a Octree) -> Option<(&'a OctreeNode,Dir)> {
    if let Some((node,t)) = ray_octree(start,dir,chunk_data) {
        let dir = hit_direction(start + dir * t,dir);
        return Some((node,dir));
    } else {
        return None;
    }
}
//struct StackNode {
    //start: Vec3, dir: Vec3, mask: u8, octree: &Octree, node_idx: i32,
    //tx0:f32,ty0:f32,tz0:f32,tx1:f32,ty1:f32,tz1:f32,
//}
//impl StackNode {
    //fn new( start: Vec3, dir: Vec3, mask: u8, octree: &Octree, node_idx: i32,
            //tx0:f32,ty0:f32,tz0:f32,tx1:f32,ty1:f32,tz1:f32) -> StackNode {
        //StackNode {
            //start, dir, mask, octree, node_idx,
            //tx0,ty0,tz0,tx1,ty1,tz1,
//
        //}
    //}
//}
pub static mut MAX_STACK_DEPTH: usize = 0;

pub fn ray_octree_stack<'a>(start: Vec3, dir: Vec3, octree: &'a Octree) -> Option<(&'a OctreeNode,f32)> {
    let mut start = start;

    let root_node_idx = 0;
    let node = &octree.nodes[root_node_idx];

    let node_pos = node.position;

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
        return None;
    }
    
    let mut stack: Vec<(i32,f32,f32,f32,f32,f32,f32)> = Vec::new();
    stack.push((root_node_idx as i32, tx0, ty0, tz0, tx1, ty1, tz1));
    
    while let Some((node_idx,tx0,ty0,tz0,tx1,ty1,tz1)) = stack.pop() {
        let t1_is_pos = tx1 >= 0. && ty1 >= 0. && tz1 >= 0. ;

        if !t1_is_pos {
            continue;
        }

        let node = &octree.nodes[node_idx as usize];
        if !node.has_children {
            if node.is_full && t1_is_pos {
                if tx0 < 0. && ty0 < 0. && tz0 < 0.{
                    return Some((node,0.));
                }else {
                    let t_min = tx0.max(ty0).max(tz0);
                    return Some((node,t_min));
                }
            } else {
                continue;
            }
        }

        let txm = (tx0 + tx1) /2.;
        let tym = (ty0 + ty1) /2.;
        let tzm = (tz0 + tz1) /2.;

        let mut curr_node = first_node(tx0,ty0,tz0,txm,tym,tzm);

        let children_idx = node.children_idx;
        //println!("hit index {curr_node}");
        let mut node_buffer = Vec::new();

        while curr_node < 8 {
            let child_idx = children_idx[curr_node as usize ^ mask as usize];
            match curr_node {
                0 => {
                    node_buffer.push((child_idx, tx0, ty0, tz0, txm, tym, tzm));
                    curr_node = next_node(curr_node,txm,tym,tzm);
                },
                1 => {
                    node_buffer.push((child_idx, tx0,ty0,tzm,txm,tym,tz1));
                    curr_node = next_node(curr_node,txm,tym,tz1);
                },
                2 => {
                    node_buffer.push((child_idx, tx0, tym, tz0, txm, ty1, tzm));
                    curr_node = next_node(curr_node,txm,ty1,tzm);
                },
                3 => {
                    node_buffer.push((child_idx, tx0, tym, tzm, txm, ty1, tz1));
                    curr_node = next_node (curr_node,txm, ty1, tz1,);
                },
                4 => {
                    node_buffer.push((child_idx, txm, ty0, tz0, tx1, tym, tzm));
                    curr_node = next_node (curr_node,tx1,  tym,  tzm);
                },
                5 => {
                    node_buffer.push((child_idx, txm, ty0, tzm, tx1, tym, tz1));
                    curr_node = next_node ( curr_node,tx1,  tym,  tz1);
                },
                6 => {
                    node_buffer.push((child_idx, txm, tym, tz0, tx1, ty1, tzm));
                    curr_node = next_node ( curr_node,tx1,  ty1,  tzm );
                },
                7 => {
                    stack.push((child_idx, txm, tym, tzm, tx1, ty1, tz1 ));
                    curr_node = 8;
                },
                _ => panic!(),
            }
        }
        unsafe {
            if stack.len() - 1 >  MAX_STACK_DEPTH {
                MAX_STACK_DEPTH = stack.len() -1;
            }
        }
        while let Some(args) = node_buffer.pop() {
            stack.push(args); 
        }
    }
    return None;
}
pub fn ray_octree_max<'a>(start: Vec3, dir: Vec3, octree: &'a Octree, max: f32) -> Option<(&'a OctreeNode,f32)> {
    let mut start = start;

    let root_node_idx = 0;
    let node = &octree.nodes[root_node_idx];

    let node_pos = node.position;

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
        return None;
    }

    return proc_subtree(start,dir,mask,octree,max,root_node_idx as i32,tx0,ty0,tz0,tx1,ty1,tz1);

    fn proc_subtree(start: Vec3, dir: Vec3, mask: u8,octree: &Octree, max: f32,node_idx: i32,
                    tx0:f32,ty0:f32,tz0:f32,tx1:f32,ty1:f32,tz1:f32) -> Option<(&OctreeNode,f32)> {
        let t1_is_pos = tx1 >= 0. && ty1 >= 0. && tz1 >= 0. ;

                     
        if !t1_is_pos || tx0.max(ty0).max(tz0) > max {
            return None;
        }

        let node = &octree.nodes[node_idx as usize];
        if !node.has_children {
            if node.is_full && t1_is_pos {
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

        let children_idx = node.children_idx;
        //println!("hit index {curr_node}");

        while curr_node < 8 {
            let child_idx = children_idx[curr_node as usize ^ mask as usize];
            match curr_node {
                0 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, tx0, ty0, tz0, txm, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tzm);
                },
                1 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, tx0,ty0,tzm,txm,tym,tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tz1);
                },
                2 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, tx0, tym, tz0, txm, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,ty1,tzm);
                },
                3 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, tx0, tym, tzm, txm, ty1, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,txm, ty1, tz1,);
                },
                4 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, txm, ty0, tz0, tx1, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,tx1,  tym,  tzm);
                },
                5 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, txm, ty0, tzm, tx1, tym, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  tym,  tz1);
                },
                6 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, txm, tym, tz0, tx1, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  ty1,  tzm );
                },
                7 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,max,child_idx, txm, tym, tzm, tx1, ty1, tz1 ) {
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

pub fn ray_octree<'a>(start: Vec3, dir: Vec3, octree: &'a Octree) -> Option<(&'a OctreeNode,f32)> {
    let mut start = start;

    let root_node_idx = 0;
    let node = &octree.nodes[root_node_idx];

    let node_pos = node.position;

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
        return None;
    }

    return proc_subtree(start,dir,mask,octree,root_node_idx as i32,tx0,ty0,tz0,tx1,ty1,tz1);

    fn proc_subtree(start: Vec3, dir: Vec3, mask: u8,octree: &Octree, node_idx: i32,
                    tx0:f32,ty0:f32,tz0:f32,tx1:f32,ty1:f32,tz1:f32) -> Option<(&OctreeNode,f32)> {
        let t1_is_pos = tx1 >= 0. && ty1 >= 0. && tz1 >= 0. ;

        if !t1_is_pos {
            return None;
        }

        let node = &octree.nodes[node_idx as usize];
        if !node.has_children {
            if node.is_full && t1_is_pos {
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

        let children_idx = node.children_idx;
        //println!("hit index {curr_node}");

        while curr_node < 8 {
            let child_idx = children_idx[curr_node as usize ^ mask as usize];
            match curr_node {
                0 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, tx0, ty0, tz0, txm, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tzm);
                },
                1 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, tx0,ty0,tzm,txm,tym,tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,tym,tz1);
                },
                2 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, tx0, tym, tz0, txm, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node(curr_node,txm,ty1,tzm);
                },
                3 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, tx0, tym, tzm, txm, ty1, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,txm, ty1, tz1,);
                },
                4 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, txm, ty0, tz0, tx1, tym, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node (curr_node,tx1,  tym,  tzm);
                },
                5 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, txm, ty0, tzm, tx1, tym, tz1) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  tym,  tz1);
                },
                6 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, txm, tym, tz0, tx1, ty1, tzm) {
                        return Some((hit,t));
                    }
                    curr_node = next_node ( curr_node,tx1,  ty1,  tzm );
                },
                7 => {
                    if let Some((hit,t)) = proc_subtree(start, dir, mask, octree,child_idx, txm, tym, tzm, tx1, ty1, tz1 ) {
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

    let t_delta = Vec3::new( 
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

    let t_delta = Vec3::new( 
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
            if chunk_data[voxel.x as usize][voxel.y as usize][voxel.z as usize] != 0 {
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
