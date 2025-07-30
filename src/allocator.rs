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
        let err = gl::GetError();
        if err != gl::NO_ERROR {
            panic!("OpenGL error: 0x{:X}", err);
        } else {
            println!("Buffer created successfully.");
        }


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
        self.print_state();
        println!("allocating {} bricks => {}MiB",len_to_alloc, len_to_alloc * core::mem::size_of::<Brick>() / (1024*1024));
        println!("space used {}MiB",self.used_memory() * core::mem::size_of::<Brick>() / 1024 / 1024);
        let mut free_block_list = self.free_block_list.lock().unwrap();
        consolidate_free_blocks(&mut free_block_list);

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
                free_block.len   -= len_to_alloc;
            };

            return Ok(BrickVec::new(
                old_free_block_start,
                len_to_alloc,
            ));
        }

        return Err(format!("No free block with sufficient space"));
    }
    pub fn free(&self, brick_vec: BrickVec) {
        self.free_block_list.lock().unwrap().push( 
            FreeBlock { 
                start: brick_vec.start     as usize, 
                len:   brick_vec.capacity  as usize,
        });
    }
    pub fn used_memory(&self) -> usize {
        let mut sum = 0;
        let total = self.max_len;
        for block in self.free_block_list.lock().unwrap().iter() {
            sum += block.len;
        }
        return total - sum;
    }
    pub fn print_state(&self) {
        let width = 120;
        let mut buffer = vec!['#'; width];

        let buffer_size = self.max_len;

        for block in self.free_block_list.lock().unwrap().iter() {
            let start_ratio = block.start as f64 / buffer_size as f64;
            let end_ratio = (block.start + block.len) as f64 / buffer_size as f64;

            let scaled_start = (start_ratio * width as f64).floor() as usize;
            let scaled_end = (end_ratio * width as f64).ceil() as usize;

            for i in scaled_start..scaled_end.min(width) {
                buffer[i] = '.';
            }
        }

        // Now insert '|' markers (insert = shift right)
        let mut marker_positions: Vec<usize> = self.free_block_list.lock().unwrap()
            .iter()
            .map(|block| {
                let start_ratio = block.start as f64 / buffer_size as f64;
                (start_ratio * width as f64).floor() as usize
            })
            .collect();

        // Sort and dedup marker positions to prevent multiple inserts at same location
        marker_positions.sort_unstable();
        marker_positions.dedup();

        for (i, pos) in marker_positions.iter().enumerate() {
            // Adjust insertion point based on how many have already been inserted
            let adjusted_pos = pos + i;
            if adjusted_pos <= buffer.len() {
                buffer.insert(adjusted_pos, '|');
            }
        }

        let visualization: String = buffer.into_iter().collect();
        println!("{}", visualization);
    }
}

pub fn consolidate_free_blocks(list: &mut Vec<FreeBlock>) {
    list.sort_by_key(|k| k.start);

    let mut i = 0;
    while i < list.len() - 1 {
        if list[i].start + list[i].len == list[i+1].start {
            let b_len = list[i+1].len;
            println!("merging Blocks");
            list.remove(i+1);
            list[i].len += b_len;
        }
        i+=1;
    }
}

pub struct BrickVec {
    ///Start index in the BrickAllocator
    pub start:      u32,
    pub len:        u32,
    pub capacity:   u32,
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
        let offset = self.start as isize * mem::size_of::<Brick>() as isize;
        let size = self.capacity as isize * mem::size_of::<Brick>() as isize;
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, BRICK_ALLOCATOR().data_ssbo);
        gl::BufferSubData(
            gl::SHADER_STORAGE_BUFFER,
            offset,
            size,
            src_ptr as *const _,
        );
    }
    pub fn from_vec(vec: Vec<Brick>) -> Result<BrickVec,String> {
        let mut brick_vec = BRICK_ALLOCATOR().alloc(vec.len());
        if let Err(err) = brick_vec {
            use crate::colors::*;
            println!("{}{}{}",RED,err.to_uppercase(),RESET_COL);
            return Err(err);
        };
        let mut brick_vec = brick_vec.unwrap();

        brick_vec.len = vec.len() as u32;
        let brick_vec_ptr: *const Brick = unsafe { BRICK_ALLOCATOR().data.as_ptr().add(brick_vec.start as usize) };

        unsafe { ptr::copy_nonoverlapping(vec.as_ptr(),brick_vec_ptr as *mut Brick ,vec.len()) };
        Ok(brick_vec)
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
