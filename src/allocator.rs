use crate::chunk::Brick;
use std::sync::{OnceLock,Mutex};
use core::mem::MaybeUninit;
use core::ptr;
use std::mem;

static G_BRICK_ALLOCATOR: OnceLock<BrickAllocator> = OnceLock::new();

pub fn BRICK_ALLOCATOR() -> &'static BrickAllocator {
    G_BRICK_ALLOCATOR.get().expect("G_BRICK_ALLOCATOR not initialized")
}

pub struct FreeBlock {
    start: usize,
    /// len in Bricks
    len: usize,
}

pub struct BrickAllocator {
    pub data: Box<[Brick]>,
    pub free_block_list: Mutex<Vec<FreeBlock>>,
    pub max_len: usize,
    pub data_ssbo: u32,
}
impl BrickAllocator {
    pub unsafe fn init(num_of_bricks: u32) {
        if G_BRICK_ALLOCATOR.set(BrickAllocator::create(num_of_bricks as usize)).is_err() {
            panic!("G_BRICK_ALLOCATOR already initialized");
        }
    }
    pub unsafe fn create(num_of_bricks: usize) -> Self {
        use std::mem;

        let mut ssbo = 0;
        let buffer_size = num_of_bricks * mem::size_of::<Brick>();

        gl::GenBuffers(1, &mut ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            buffer_size as isize,
            std::ptr::null(), // no initial data
            gl::DYNAMIC_DRAW,
        );

        // Make an unititialized allocation on the heap
        // and cast to Box<[Brick]>, the data is never read directly
        // only thru BrickVec's so its fine
        let data = unsafe {
            let mut vec: Vec<MaybeUninit<Brick>> = Vec::with_capacity(num_of_bricks);
            vec.set_len(num_of_bricks);
            let boxed: Box<[MaybeUninit<Brick>]> = vec.into_boxed_slice();
            Box::from_raw(Box::into_raw(boxed) as *mut [Brick])
        };

        // FreeBlock thats the size of the Buffer
        let free_block = FreeBlock {
            start: 0,
            len: num_of_bricks,
        };
        
        Self {
            data,
            max_len: num_of_bricks,
            data_ssbo: ssbo,
            free_block_list: Mutex::new(vec![free_block]),
        }
    }
    pub fn alloc(&self, len_to_alloc: usize) -> Result<BrickVec,String> {
        let mut free_block_list = self.free_block_list.lock().unwrap();

        let mut i = 0;
        while i < free_block_list.len() {
            let free_block = &mut free_block_list[i];

            if free_block.len < len_to_alloc {
                i += 1;
                continue; 
            }
            let old_free_block_start  = free_block.start;
            // Add to the start of the free block 'len_to_alloc'
            if free_block.len == len_to_alloc {
                free_block_list.remove(i);
            } else {
                free_block.start += len_to_alloc;
            };

            return Ok(BrickVec::new(
                old_free_block_start,
                len_to_alloc,
            ));
        }

        return Err(format!("No free block with sufficient space"));
    }
    pub fn free(&mut self, brick_vec: BrickVec) {
        self.free_block_list.lock().unwrap().push( 
            FreeBlock { 
                start: brick_vec.start     as usize, 
                len:   brick_vec.capacity  as usize,
        });
    }
}

pub struct BrickVec {
    ///Start index in the BrickAllocator
    start:      u32,
    len:        u32,
    capacity:   u32,
}
impl BrickVec {
    pub fn new(start: usize, capacity: usize) -> BrickVec {
        BrickVec {
            start: start as u32,
            len: 0,
            capacity: capacity as u32,
        }
    }
    /// Sends the data to the GPU ssbo
    pub unsafe fn send(&self) {
        let src_ptr = BRICK_ALLOCATOR().data.as_ptr().add(self.start as usize);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, BRICK_ALLOCATOR().data_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            self.len as isize * mem::size_of::<Brick>() as isize,
            src_ptr as *const _,
            gl::DYNAMIC_DRAW,
        );
    }
    pub fn from_vec(vec: Vec<Brick>) -> BrickVec {
        let brick_vec = BRICK_ALLOCATOR().alloc(vec.len()).unwrap();
        let brick_vec_ptr: *const Brick = unsafe { BRICK_ALLOCATOR().data.as_ptr().add(brick_vec.start as usize) };

        unsafe { ptr::copy_nonoverlapping(vec.as_ptr(),brick_vec_ptr as *mut Brick ,vec.len()) };
        brick_vec
    }
}
impl std::ops::Index<usize> for BrickVec {
    type Output = Brick;

    fn index(&self, index: usize) -> &Self::Output {
        &BRICK_ALLOCATOR().data[index]
    }
}
impl std::ops::IndexMut<usize> for BrickVec {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            &mut *(BRICK_ALLOCATOR().data.as_ptr().add(index) as *mut Brick)
        }
    }
}
