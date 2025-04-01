use std::ptr::null_mut;
use std::ptr;
use std::str;
use std::ffi::CString;

use gl33::*;
use gl33::global_loader::*;

use my_math::vec::Vec3;

#[derive(Clone,Copy)]
pub struct ShaderProgram(u32);
impl ShaderProgram {
    pub unsafe fn create_program(vs: u32, fs: u32) -> ShaderProgram {
        let mut success:i32 = 0;
        let mut infolog: [u8;512]= [0;512];

        let program = glCreateProgram();
        glAttachShader(program, vs);
        glAttachShader(program, fs);

        glLinkProgram(program);

        glGetProgramiv(program, GL_LINK_STATUS, &mut success );
        if success == 0 {
            glGetProgramInfoLog(program, 512, null_mut(), infolog.as_mut_ptr());
            panic!("shader linking error: {}", str::from_utf8(&infolog).unwrap());
        }

        ShaderProgram(program)
    }
    pub unsafe fn set_vec3(self,name: &str,val: Vec3) {
        glUniform3f(GetUniformLocation(self.0,name), val.x, val.y, val.z);
    }
    pub unsafe fn set_mat4(self,name: &str, ptr: *const f32) {
        glUniformMatrix4fv(GetUniformLocation(self.0,name), 1, 0, ptr);
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
    let out = glGetUniformLocation(program, CString::new(name).unwrap().as_ptr() as *const _);
    if out == -1 {
        println!("COULDNT FIND UNIFORM: {}",name) ;
    }
    out
}


// Function to compile a shader
pub unsafe fn compile_shader(shader_type: GLenum, path: &str) -> u32 {
    fn read_shader(path: &str) -> String {
        std::fs::read_to_string(path).expect(&format!("Failed to read shader file: {}", path)) + "\0"
    }

    let mut succcess:i32 = 0;
    let mut infolog: [u8;512]= [0;512];

    let source = read_shader(path);
    let shader = glCreateShader(shader_type);

    let s = &(source.as_ptr() as *const u8) as *const *const u8;
    glShaderSource(shader, 1, s, ptr::null());
    glCompileShader(shader);

    glGetShaderiv(shader, GL_COMPILE_STATUS, &mut succcess);
    if succcess == 0 {
        glGetShaderInfoLog(shader, 512, null_mut(), infolog.as_mut_ptr());
        let shader_type = match shader_type {
            GL_VERTEX_SHADER =>     "vertex ",
            GL_FRAGMENT_SHADER =>   "fragment ",
            _ =>                    "",
        };
        panic!("{shader_type}shader compilation error: {}", str::from_utf8(&infolog).unwrap());
    }

    shader
}
