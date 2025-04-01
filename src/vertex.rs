use gl33::global_loader::*;
use gl33::*;

use my_math::vec::Vec3;
use std::ptr::null_mut;
use core::mem::size_of;

#[derive(Debug,Copy,Clone)]
pub struct Vertex {
    pub pos: Vec3,
    pub col: Vec3,
    pub norm: Vec3,
}
impl VertexAttributes for Vertex {
    unsafe fn set_attribs() {
        // index is the location in the shader
        // position
        glVertexAttribPointer( 0, 3, GL_FLOAT, 0, size_of::<Vertex>() as i32, null_mut(),);
        glEnableVertexAttribArray(0);
        // color
        glVertexAttribPointer( 1, 3, GL_FLOAT, 0, size_of::<Vertex>() as i32, (size_of::<f32>() * 3) as *const _);
        glEnableVertexAttribArray(1);

        glVertexAttribPointer( 2, 3, GL_FLOAT, 0, size_of::<Vertex>() as i32, (size_of::<f32>() * 6) as *const _);
        glEnableVertexAttribArray(2);
    }

}
pub trait VertexAttributes {
    unsafe fn set_attribs();
}
