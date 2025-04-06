use my_math::vec::Vec3;
use my_math::vec::Vec2;
use std::ptr::null_mut;
use core::mem::size_of;

pub trait VertexAttributes {
    unsafe fn set_attribs();
}

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub struct UvVertex {
    pub pos: Vec2,
    pub uv_pos: Vec2,
}
impl VertexAttributes for UvVertex {
    unsafe fn set_attribs() {
        // index is the location in the shader
        // pos
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, size_of::<UvVertex>() as i32, null_mut()); // Position
        gl::EnableVertexAttribArray(0);
        // tex_cords
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, size_of::<UvVertex>() as i32, (2 * size_of::<f32>()) as *const _); // Texture Coords
        gl::EnableVertexAttribArray(1);
    }

}

#[repr(C)]
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
        gl::VertexAttribPointer( 0, 3, gl::FLOAT, 0, size_of::<Vertex>() as i32, null_mut(),);
        gl::EnableVertexAttribArray(0);
        // color
        gl::VertexAttribPointer( 1, 3, gl::FLOAT, 0, size_of::<Vertex>() as i32, (size_of::<f32>() * 3) as *const _);
        gl::EnableVertexAttribArray(1);

        gl::VertexAttribPointer( 2, 3, gl::FLOAT, 0, size_of::<Vertex>() as i32, (size_of::<f32>() * 6) as *const _);
        gl::EnableVertexAttribArray(2);
    }

}
