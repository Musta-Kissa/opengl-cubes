use std::ptr::null_mut;
use std::ptr;
use std::str;
use std::ffi::CString;

//use gl33::*;
//use gl33::global_loader::*;

use my_math::vec::Vec3;

#[derive(Clone,Copy)]
pub struct ShaderProgram(u32);
impl ShaderProgram {
    pub unsafe fn create_program(vs: u32, fs: u32) -> ShaderProgram {
        let mut success:i32 = 0;
        let mut infolog: [u8;512]= [0;512];

        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);

        gl::LinkProgram(program);

        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success );
        if success == 0 {
            gl::GetProgramInfoLog(program, 512, null_mut(), infolog.as_mut_ptr() as *mut _);
            panic!("shader linking error: {}", str::from_utf8(&infolog).unwrap());
        }

        ShaderProgram(program)
    }
    pub unsafe fn create_compute(cs: u32) -> ShaderProgram {
        let mut success:i32 = 0;
        let mut infolog: [u8;512]= [0;512];

        let program = gl::CreateProgram();
        gl::AttachShader(program, cs);

        gl::LinkProgram(program);

        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success );
        if success == 0 {
            gl::GetProgramInfoLog(program, 512, null_mut(), infolog.as_mut_ptr() as *mut i8);
            panic!("shader linking error: {}", std::str::from_utf8(&infolog).unwrap());
        }

        ShaderProgram(program)
    }
    pub unsafe fn set_vec3(self,name: &str,val: Vec3) {
        gl::Uniform3f(GetUniformLocation(self.0,name), val.x, val.y, val.z);
    }
    pub unsafe fn set_mat4(self,name: &str, ptr: *const f32) {
        gl::UniformMatrix4fv(GetUniformLocation(self.0,name), 1, 0, ptr);
    }
}
use std::ops::Deref;
impl Deref for ShaderProgram {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}

#[allow(non_snake_case,warnings)]
pub unsafe fn GetUniformLocation(program: u32,name: &str) -> i32 {
    let out = gl::GetUniformLocation(program, CString::new(name).unwrap().as_ptr() as *const _);
    if out == -1 {
        println!("COULDNT FIND UNIFORM: {}",name) ;
    }
    out
}


// Function to compile a shader
pub unsafe fn compile_shader(shader_type: u32, path: &str) -> u32 {
    fn read_shader(path: &str) -> String {
        std::fs::read_to_string(path).expect(&format!("Failed to read shader file: {}", path)) + "\0"
    }

    let mut succcess:i32 = 0;
    let mut infolog: [u8;512]= [0;512];

    let source = read_shader(path);
    let shader = gl::CreateShader(shader_type);

    let s = &(source.as_ptr() as *const i8) as *const *const i8;
    gl::ShaderSource(shader, 1, s, ptr::null());
    gl::CompileShader(shader);

    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut succcess);
    if succcess == 0 {
        gl::GetShaderInfoLog(shader, 512, null_mut(), infolog.as_mut_ptr() as *mut _);
        let shader_type = match shader_type {
            gl::VERTEX_SHADER =>     "vertex ",
            gl::FRAGMENT_SHADER =>   "fragment ",
            _ =>                    "",
        };
        panic!("{shader_type}shader compilation error: {}", str::from_utf8(&infolog).unwrap());
    }

    shader
}
