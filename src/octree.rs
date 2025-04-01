use my_math::prelude::*;
use gl33::*;
use gl33::global_loader::*;

use crate::mesh::gen_cube_skeleton;
use crate::mesh::gen_cube;
use crate::mesh::Mesh;

pub struct OctreeNode {
    pub is_full: bool,

    pub children: Option<[Box<OctreeNode>;8]>,
    pub size: i32,
    pub position: IVec3,
}
impl OctreeNode {
    pub fn new(size: i32,pos: IVec3,full: bool) -> Self {
        OctreeNode {
            is_full: full,

            children: None,
            size: size,
            position: pos,
        }
    }
    pub fn gen_skeleton_mesh(&self,mesh:&mut Mesh) {
        if let Some(children) = &self.children {
            for child in children {
                child.gen_skeleton_mesh(mesh);
            }
        }
        mesh.join_with(&gen_cube_skeleton(self.size,self.position));
    }
    pub fn gen_mesh(&self, mesh: &mut Mesh) {
        if let Some(children) = &self.children {
            for child in children {
                child.gen_mesh(mesh);
            }
        } else if self.is_full {
            mesh.join_with(&gen_cube(self.size,self.position.into(),vec3!(0.,0.,0.7)));
        }
    }
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }
    /// Assuming the pos is inside node
    pub fn remove_block(&mut self, pos: IVec3) {
        if self.size == 1 {
            assert!(self.children.is_none());
            self.is_full = false;
            return;
        }
        if self.children.is_none() && !self.is_full {
            // isn't full and a leaf => is empty => nothing to do
            return;
        }
        if self.children.is_none() {
            // is full and a leaf => devide and call recursively on the proper node
            self.devide(true);

            let child_idx = pos_in_child(self,pos);

            let children = self.children.as_mut().unwrap();
            children[child_idx].remove_block(pos);

            self.is_full = false;
            return;
        } else {
            // is mixed so not full and has children => just call recursively and check if removed
            // the last child if so merge the four nodes into an empty node
            let child_idx = pos_in_child(self,pos);
            let children = self.children.as_mut().unwrap();
            children[child_idx].remove_block(pos);

            // if any child after removing is full return else merge
            for child in children {
                if child.is_full || child.children.is_some() {
                    return;
                }
            }
            // its already not full so just remove the children and return
            self.children = None;
            return;
        }
    }
    pub fn add_block(&mut self, pos: IVec3) {
        if self.size == 1 {
            self.is_full = true;
            return;
        } 
        if self.is_full {
            assert!(self.children.is_none());
            return;
        }
        if self.children.is_none() {
            // is a leaf and not full so its empty => devide and call recursively
            assert!(self.children.is_none());
            self.devide(false);

            let child_idx = pos_in_child(self,pos);
            let children = self.children.as_mut().unwrap();

            children[child_idx].add_block(pos);

            // is empty so one additional voxel cant make it full
            return;
        } else {
            // isn't a leaf and isn't full => dont devide call recursively and check if filled if
            // so merge
            let child_idx = pos_in_child(self,pos);
            let children = self.children.as_mut().unwrap();

            children[child_idx].add_block(pos);

            // check if full 
            for child in children {
                if !child.is_full {
                    return;
                }
            }
            self.is_full = true;
            self.children = None;
        }
    }
    pub fn devide(&mut self,full: bool) {
        //self.is_leaf = false;

        let half_size = self.size /2;
        let pos = self.position;

        self.children = Some ([ 
                // q0
                Box::new(OctreeNode::new(half_size, pos, full)),                                       
                // q1
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x , pos.y , pos.z + half_size),full)),        
                // q2
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x , pos.y + half_size, pos.z ),full)),        
                // q3
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x , pos.y + half_size, pos.z + half_size),full)),        
                // q4
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x + half_size, pos.y , pos.z ),full)),        
                // q5
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x + half_size, pos.y , pos.z + half_size),full)),        
                // q6
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x + half_size, pos.y + half_size, pos.z ),full)),        
                // q7
                Box::new(OctreeNode::new(half_size,
                                    ivec3!(pos.x + half_size, pos.y + half_size, pos.z + half_size),full)),        
            ])
    }
}
/// Assuming the pos is inside the node
pub fn pos_in_child(node: &OctreeNode,pos:IVec3) -> usize {
    let rel_pos = pos - node.position;
    let half_size = node.size / 2;
    //if !inside_bouds(node,pos) {
        //return 8;
    //}

    let middle = ivec3!(half_size,half_size,half_size);

    if rel_pos.x < middle.x {
        // is in left half (looking +Z)
        if rel_pos.y < middle.y {
            // and is in bottom half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return 0;
            } else {
                return 1;
            }
        } else {
            // and is in top half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return 2;
            } else {
                return 3;
            }

        }
    } else {
        // is in right half (looking +Z)
        if rel_pos.y < middle.y {
            // and is in bottom half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return 4;
            } else {
                return 5;
            }
        } else {
            // and is in top half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return 6;
            } else {
                return 7;
            }

        }
    }
}
pub fn inside_bouds(node: &OctreeNode, pos: IVec3) -> bool {
    let node_pos = node.position;
    return !(pos.x < node_pos.x || pos.x >= node_pos.x + node.size ||
             pos.y < node_pos.y || pos.y >= node_pos.y + node.size ||
             pos.z < node_pos.z || pos.z >= node_pos.z + node.size)
}
pub struct Octree {
    pub head: OctreeNode
}
impl Octree {
    pub fn new(size: i32, pos: IVec3) -> Self {
        let mut s = size ;
        while s != 1 {
            assert!(s % 2 == 0, "the size of the quad tree must be a power of two");
            s /= 2;
        }
        Octree {
            head: OctreeNode::new(size,pos,false),
        }
    }
    pub fn new_full(size: i32, pos: IVec3) -> Self {
        let mut s = size ;
        while s != 1 {
            assert!(s % 2 == 0, "the size of the quad tree must be a power of two");
            s /= 2;
        }
        Octree {
            head: OctreeNode::new(size,pos,true),
        }
    }
    pub fn add_block(&mut self,pos:IVec3) {
        if inside_bouds(&self.head,pos){
            self.head.add_block(pos);
        }
    }
    pub fn remove_block(&mut self,pos:IVec3) {
        if inside_bouds(&self.head,pos){
            self.head.remove_block(pos);
        }
    }
    pub fn is_solid_at(&self,pos:IVec3) -> bool {
        let mut curr = &self.head;
        if !inside_bouds(curr,pos) {
            return false;
        }
        loop {
            if curr.is_full {
                return true;
            }
            if curr.children.is_none() || curr.size == 1 {
                return false;
            }

            let children = curr.children.as_ref().unwrap();
            let child_idx = pos_in_child(curr,pos);
            curr = &children[child_idx];
        }
    }
    pub fn gen_skeleton_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new();
        self.head.gen_skeleton_mesh(&mut mesh);
        mesh
    }
    pub fn gen_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new();
        self.head.gen_mesh(&mut mesh);
        mesh
    }
    /*
    pub fn index_at(&self, pos: IVec2) -> i32 {
        todo!();
        if pos.x < self.head.position.x || pos.x >= self.head.position.x + self.head.size ||
            pos.y < self.head.position.y || pos.y >= self.head.position.y + self.head.size {
            return 1;
        }
        let mut curr = &self.head;
        let mut curr_idx = -1;
        loop {
            if curr.is_full || curr.children.is_none() {
                return curr_idx;
            }
            if curr.size == 1 {
                return curr_idx;
            }

            let children = curr.children.as_ref().unwrap();
            let rel_pos = pos - curr.position;
            if rel_pos.x < curr.size/2 && rel_pos.y < curr.size/2 {
                curr = &children[0];
                curr_idx = 0;
            } else if rel_pos.x < curr.size && rel_pos.y < curr.size/2 {
                curr = &children[1];
                curr_idx = 1;
            } else if rel_pos.x < curr.size/2 && rel_pos.y < curr.size {
                curr = &children[2];
                curr_idx = 2;
            } else {
                curr = &children[3];
                curr_idx = 3;
            }
        }
    }
    pub fn size_at(&self, pos: IVec2) -> i32 {
        todo!();
        if pos.x < self.head.position.x || pos.x >= self.head.position.x + self.head.size ||
            pos.y < self.head.position.y || pos.y >= self.head.position.y + self.head.size {
            return 1;
        }
        let mut curr = &self.head;
        loop {
            if curr.is_full || curr.children.is_none() {
                return curr.size;
            }
            if curr.size == 1 {
                return 1;
            }

            let children = curr.children.as_ref().unwrap();
            let rel_pos = pos - curr.position;
            if rel_pos.x < curr.size/2 && rel_pos.y < curr.size/2 {
                curr = &children[0];
            } else if rel_pos.x < curr.size && rel_pos.y < curr.size/2 {
                curr = &children[1];
            } else if rel_pos.x < curr.size/2 && rel_pos.y < curr.size {
                curr = &children[2];
            } else {
                curr = &children[3];
            }
        }
    }
    */
}
