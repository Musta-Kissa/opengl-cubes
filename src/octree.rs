use my_math::prelude::*;

use crate::mesh::gen_cube_skeleton;
use crate::mesh::gen_cube;
use crate::mesh::Mesh;
use crate::vertex::Vertex;

#[repr(C)]
#[derive(Clone)]
//pub is_orphan: bool,
pub struct OctreeNode {
    pub children_idx: [i32;8],
    pub size: u32,

    pub is_full: bool,
    pub _padding0: [u8;3],

    pub has_children: bool,
    pub _padding1: [u8;3],

    pub position: IVec3,
}
impl OctreeNode {
    pub fn new(size: u32,pos: IVec3,full: bool) -> Self {
        //is_orphan: full,
        OctreeNode {
            children_idx: [0;8],
            size: size,

            is_full: full,
            _padding1: [0;3],

            has_children: false,
            _padding0: [0;3],

            position: pos,
        }
    }
}
const ROOT_IDX:usize = 0;
pub struct Octree {
    pub nodes: Vec<OctreeNode>
}
impl Octree {
    pub fn new(size: u32, pos: IVec3) -> Self {
        let mut s = size ;
        while s != 1 {
            assert!(s % 2 == 0, "the size of the quad tree must be a power of two");
            s /= 2;
        }
        Octree {
            nodes: vec![OctreeNode::new(size,pos,false)],
        }
    }
    pub fn new_full(size: u32, pos: IVec3) -> Self {
        let mut s = size ;
        while s != 1 {
            assert!(s % 2 == 0, "the size of the quad tree must be a power of two");
            s /= 2;
        }
        Octree {
            nodes: vec![OctreeNode::new(size,pos,true)],
        }
    }

    fn devide_node(&mut self, node_idx: i32,full: bool) {
        let len = self.nodes.len();
        let node = &self.nodes[node_idx as usize];

        let half_size = (node.size / 2) as i32;
        let pos = node.position;
        let nodes = [
                OctreeNode::new(half_size as u32,        pos                                                     ,full), 
                OctreeNode::new(half_size as u32, ivec3!(pos.x            , pos.y            , pos.z + half_size),full),
                OctreeNode::new(half_size as u32, ivec3!(pos.x            , pos.y + half_size, pos.z            ),full),
                OctreeNode::new(half_size as u32, ivec3!(pos.x            , pos.y + half_size, pos.z + half_size),full),
                OctreeNode::new(half_size as u32, ivec3!(pos.x + half_size, pos.y            , pos.z            ),full),
                OctreeNode::new(half_size as u32, ivec3!(pos.x + half_size, pos.y            , pos.z + half_size),full),
                OctreeNode::new(half_size as u32, ivec3!(pos.x + half_size, pos.y + half_size, pos.z            ),full),
                OctreeNode::new(half_size as u32, ivec3!(pos.x + half_size, pos.y + half_size, pos.z + half_size),full),
        ];

        // TODO remove orphan nodes
        self.nodes.extend_from_slice(&nodes);
        self.nodes[node_idx as usize].children_idx = [0,1,2,3,4,5,6,7].map(|x| (len + x) as i32);
        return;
    }

    pub fn add_block(&mut self,pos:IVec3) -> bool {
        if !inside_bouds(&self.nodes[ROOT_IDX],pos) {
            return false;
        }

        unsafe { add_block_recursion(self,pos,ROOT_IDX as i32) };
        return true;

        unsafe fn add_block_recursion(tree: &mut Octree,pos: IVec3,node_idx:i32) {
            //println!("add block {:?}",node_idx);
            // so rust wont complain, makes the code cleaner no need to index into tree.nodes every time
            let curr_node: &mut OctreeNode = &mut *((&mut tree.nodes[node_idx as usize]) as *mut _);

            if curr_node.size == 1 {
                assert!(!curr_node.has_children);
                curr_node.is_full = true;
                return;
            } 
            if curr_node.is_full {
                assert!(!curr_node.has_children);
                return;
            }

            if !curr_node.has_children {
                // is a leaf and not full so its empty => devide and call recursively
                curr_node.has_children = true;
                tree.devide_node(node_idx,false);
                let curr_node: &mut OctreeNode = &mut *((&mut tree.nodes [node_idx as usize]) as *mut _);

                let child_idx = pos_to_idx(curr_node,pos);

                // is empty so one additional voxel cant make it full
                add_block_recursion(tree,pos,child_idx);
            } else {
                let children_idx = curr_node.children_idx;
                // isn't a leaf and isn't full => dont devide call recursively and check if filled if
                // so merge
                let child_idx = pos_to_idx(curr_node,pos);

                add_block_recursion(tree,pos,child_idx);
                let curr_node: &mut OctreeNode = &mut *((&mut tree.nodes[node_idx as usize]) as *mut _);

                // check if full 
                for child_idx in children_idx {
                    if !tree.nodes[child_idx as usize].is_full {
                        return;
                    }
                }
                curr_node.is_full = true;
                //curr_node.children_idx = None;
                curr_node.has_children = false
            }
        }
    }
    pub fn remove_block(&mut self,pos:IVec3) -> bool {
        if !inside_bouds(&self.nodes[ROOT_IDX],pos){
            return false;
        }

        unsafe { remove_block_recursion(self,pos,ROOT_IDX as i32); }
        return true;
        
        unsafe fn remove_block_recursion(tree: &mut Octree, pos: IVec3, node_idx: i32) {
            let curr_node: &mut OctreeNode = &mut *((&mut tree.nodes[node_idx as usize]) as *mut _);

            if curr_node.size == 1 {
                assert!(!curr_node.has_children);
                curr_node.is_full = false;
                return;
            }
            if !curr_node.has_children && !curr_node.is_full {
                // isn't full and a leaf => is empty => nothing to do
                return;
            }
            if !curr_node.has_children {
                // is full and a leaf => devide and call recursively on the proper node
                curr_node.has_children = true;
                tree.devide_node(node_idx,true);
                let curr_node: &mut OctreeNode = &mut *((&mut tree.nodes[node_idx as usize]) as *mut _);

                let child_idx = pos_to_idx(curr_node,pos);

                //let children = curr_node.children.as_mut().unwrap();
                //children[child_idx].remove_block(pos);
                curr_node.is_full = false;

                remove_block_recursion(tree,pos,child_idx);
                return;
            } else {
                let children_idx = curr_node.children_idx;
                // is mixed so not full and has children => just call recursively and check if removed
                // the last child if so merge the four nodes into an empty node
                let child_idx = pos_to_idx(curr_node,pos);
                remove_block_recursion(tree,pos,child_idx);
                let curr_node: &mut OctreeNode = &mut *((&mut tree.nodes[node_idx as usize]) as *mut _);

                // if any child after removing is full return else merge
                for child_idx in children_idx {
                    if tree.nodes[child_idx as usize].is_full || tree.nodes[child_idx as usize].has_children {
                        return;
                    }
                }
                // its already not full so just remove the children and return
                //curr_node.children_idx = None;
                curr_node.has_children = false;
                return;
            }
        }
    }
    pub fn is_solid_at(&self,pos:IVec3) -> bool {
        let head = &self.nodes[ROOT_IDX];
        if !inside_bouds(head,pos) {
            return false;
        }
        let mut curr = head;
        loop {
            if curr.is_full {
                return true;
            }
            if !curr.has_children || curr.size == 1 {
                return false;
            }

            let child_idx = pos_to_idx(curr,pos);
            curr = &self.nodes[child_idx as usize];
        }
    }
    pub fn gen_skeleton_mesh(&self) -> Mesh<Vertex> {
        let mut out = Mesh::new();
        gen_skeleton_mesh_recursion(self,&mut out,ROOT_IDX as i32);
        return out;

        fn gen_skeleton_mesh_recursion(tree: &Octree, mesh: &mut Mesh<Vertex>, node_idx: i32) {
            let node = &tree.nodes[node_idx as usize];

            if node.has_children {
                let children_idx = node.children_idx;
                for child_idx in children_idx {
                    gen_skeleton_mesh_recursion(tree,mesh,child_idx);
                }
            }
            mesh.join_with(&gen_cube_skeleton(node.size as i32,node.position));
        }
    }
    pub fn gen_mesh(&self) -> Mesh<Vertex> {
        let mut mesh = Mesh::new();
        gen_mesh_recursion(self,&mut mesh,ROOT_IDX as i32);
        return mesh;

        fn gen_mesh_recursion(tree: &Octree, mesh: &mut Mesh<Vertex>, node_idx: i32) {
            let node = &tree.nodes[node_idx as usize];
            if node.has_children {
                let children_idx = node.children_idx;
                for child_idx in children_idx {
                    gen_mesh_recursion(tree,mesh,child_idx);
                }
            } else if node.is_full {
                mesh.join_with(&gen_cube(node.size as i32,node.position.into(),vec3!(0.,0.,0.7)));
            }
        }
    }
}
pub fn inside_bouds(node: &OctreeNode, pos: IVec3) -> bool {
    let node_pos = node.position;
    return !(pos.x < node_pos.x || pos.x >= node_pos.x + node.size as i32 ||
             pos.y < node_pos.y || pos.y >= node_pos.y + node.size as i32 ||
             pos.z < node_pos.z || pos.z >= node_pos.z + node.size as i32 )
}
/// Assuming the pos is inside the node and has children
pub fn pos_to_idx(node: &OctreeNode,pos:IVec3) -> i32 {
    let children_idx = &node.children_idx;

    let rel_pos = pos - node.position;
    let half_size = node.size / 2;

    let middle = ivec3!(half_size as i32,half_size as i32,half_size as i32);

    if rel_pos.x < middle.x {
        // is in left half (looking +Z)
        if rel_pos.y < middle.y {
            // and is in bottom half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return children_idx[0];
            } else {
                return children_idx[1];
            }
        } else {
            // and is in top half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return children_idx[2];
            } else {
                return children_idx[3];
            }

        }
    } else {
        // is in right half (looking +Z)
        if rel_pos.y < middle.y {
            // and is in bottom half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return children_idx[4];
            } else {
                return children_idx[5];
            }
        } else {
            // and is in top half (looking +Z)
            if rel_pos.z < middle.z {
                // and is in closer half (looking +Z)
                return children_idx[6];
            } else {
                return children_idx[7];
            }

        }
    }
}
