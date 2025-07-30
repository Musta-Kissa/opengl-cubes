use my_math::vec::*;
use crate::utils;

use crate::utils::GetBufferParameterui64vNV;
use crate::utils::MakeBufferResidentNV;
use crate::utils::BUFFER_GPU_ADDRESS_NV;

pub const BRICK_SIZE: usize = 8;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Voxel {
    pub data: u32,
    pub color: u32,
}

pub type Brick = [[[Voxel;BRICK_SIZE];BRICK_SIZE];BRICK_SIZE];

pub struct BrickGrid {
    pub arr: Vec<u32>,
    pub size: IVec3,
}

impl BrickGrid {
    pub fn new(size: IVec3) -> Self {
        let len = (size.x * size.y * size.z) as usize;
        Self { arr: vec![u32::MAX; len], size }
    }
    pub fn at(&mut self,x:usize,y:usize,z:usize) -> &mut u32 {
        let index = x * self.size.y as usize * self.size.z as usize + 
                    y * self.size.z as usize + 
                    z;
        return &mut self.arr[index];
    }
    pub fn as_ptr(&self) -> *const u32 {
        self.arr.as_ptr()
    }

    pub fn mem_size(&self) -> usize {
        4 * self.arr.len()
    }
}

#[repr(C)]
pub struct BrickMap {
    pub grid: BrickGrid,
    pub data: Vec<Brick>,
}
impl BrickMap {
    pub fn new(size: IVec3) -> Self {
        assert!(size.x % 8 == 0 &&
                size.y % 8 == 0 &&
                size.z % 8 == 0);
        Self {
            grid: BrickGrid::new(size/8),
            data: Vec::new(),
        }
    }
    pub fn add_voxel(&mut self, pos: IVec3, voxel: Voxel) {
        let grid_coords :IVec3 = pos.div_floor(8);
        let brick_coords:IVec3 = pos.modulo(8);

        let brick = self.grid.at(grid_coords.x as usize,grid_coords.y as usize,grid_coords.z as usize);

        if *brick == u32::MAX {
            *brick = self.data.len() as u32;
            let mut out = [[[ Voxel{ data: 0 , color: utils::simple_rng_u32()} ;BRICK_SIZE];BRICK_SIZE];BRICK_SIZE];
            out[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
            self.data.push(out);
        } else {
            let data = &mut self.data[*brick as usize];
            data[brick_coords.x as usize][brick_coords.y as usize][brick_coords.z as usize] = voxel;
        }
    }
    pub unsafe fn gen_ssbos(&self) -> (u32,u64,u32,u64) {
        use std::mem;
        let glGetBufferParameterui64vNV = GetBufferParameterui64vNV.unwrap();
        let glMakeBufferResidentNV = MakeBufferResidentNV.unwrap(); 

        let mut brick_grid_ssbo = 0;
        let mut brick_grid_ssbo_addr = 0;
        let mut brick_data_ssbo = 0;
        let mut brick_data_ssbo_addr = 0;

        gl::GenBuffers(1, &mut brick_grid_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_grid_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            self.grid.mem_size() as isize,
            self.grid.as_ptr() as *const _,
            gl::DYNAMIC_DRAW,
        );

        glGetBufferParameterui64vNV(gl::SHADER_STORAGE_BUFFER, BUFFER_GPU_ADDRESS_NV, &mut brick_grid_ssbo_addr);
        glMakeBufferResidentNV(gl::SHADER_STORAGE_BUFFER, gl::READ_ONLY);


        //gl::GetBufferParameterui64vNV(gl::SHADER_STORAGE_BUFFER, gl::BUFFER_GPU_ADDRESS_NV, &mut brick_grid_ssbo_addr);
        //gl::MakeBufferResidentNV(gl::SHADER_STORAGE_BUFFER, gl::READ_ONLY);
        
        // Allocate buffer for Brick data, but don't fill it yet
        let total_size = mem::size_of::<self::Brick>() * self.data.len();
        gl::GenBuffers(1, &mut brick_data_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_data_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            total_size as isize,
            std::ptr::null(), // no initial data
            gl::DYNAMIC_DRAW,
        );
        glGetBufferParameterui64vNV(gl::SHADER_STORAGE_BUFFER, BUFFER_GPU_ADDRESS_NV, &mut brick_grid_ssbo_addr);
        glMakeBufferResidentNV(gl::SHADER_STORAGE_BUFFER, gl::READ_ONLY);

        // Upload data in chunks
        let chunk_size = 1024^2;
        let brick_size = mem::size_of::<self::Brick>();
        let mut offset = 0;
        for chunk in (self.data).chunks(chunk_size) {
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
        
        (brick_grid_ssbo, brick_grid_ssbo_addr ,brick_data_ssbo,brick_data_ssbo_addr)
    }
}
