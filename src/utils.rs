use glfw::{fail_on_errors,SwapInterval,Action, Context, Glfw, Key, PRenderContext, PWindow};

use std::collections::HashMap;
use std::collections::HashSet;

use gl33::*;
use gl33::global_loader;
use gl33::global_loader::*;

use std::ptr::null_mut;
use core::mem::size_of;

use crate::vertex::*;
use crate::mesh::*;

use my_math::prelude::*;

#[derive(Clone,Copy)]
pub struct Vao {
    pub vao: u32,
    pub len: u32,
}
impl Vao {
    pub fn new(len:u32) -> Vao {
        Vao { vao: 0, len }
    }
    pub unsafe fn draw_elements(&self,draw_type: GLenum) {
        glBindVertexArray(self.vao);
        glDrawElements(draw_type, self.len as i32 ,GL_UNSIGNED_INT,null_mut());
    }
}
use std::ops::Deref;
impl Deref for Vao {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.vao
    }
}
use std::ops::DerefMut;
impl DerefMut for Vao {
    fn deref_mut(&mut self) -> &mut u32 {
        &mut self.vao
    }
}

pub unsafe fn vao_from_mesh(mesh:&Mesh) -> Vao {
    let mut vao = Vao::new(mesh.indices.len() as u32); 
    let mut vbo = 0;
    let mut ebo = 0;

    glGenVertexArrays(1, &mut *vao);
    glGenBuffers(1, &mut vbo);
    glGenBuffers(1, &mut ebo);

    glBindVertexArray(*vao);

    glBindBuffer(GL_ARRAY_BUFFER, vbo);
    glBufferData(GL_ARRAY_BUFFER,
        (mesh.verts.len() * size_of::<Vertex>()) as isize,
        mesh.verts.as_ptr() as *const _,
        GL_STATIC_DRAW,
    );

    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, ebo);
    glBufferData(GL_ELEMENT_ARRAY_BUFFER,
        (size_of::<u32>() * mesh.indices.len()) as isize, 
        mesh.indices.as_ptr() as *const _, 
        GL_STATIC_DRAW
    );
    Vertex::set_attribs();
    vao
}

pub struct InputTracker {
    pub previous_frame: HashMap<Key,Action>,
    pub pressed: HashSet<Key>,
    pub just_pressed: HashSet<Key>,
    pub just_released: HashSet<Key>,
    pub mouse_pos: Option<(f64,f64)>,
    pub mouse_delta: (f64,f64),
}
impl InputTracker {
    pub fn new() -> Self {
        Self {
            previous_frame: HashMap::new(),
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
            mouse_pos: None,
            mouse_delta: (0.,0.),
        }
    }
    pub fn update(&mut self,window: &PWindow) {
        for key in ALL_KEYS {
            let action = window.get_key(*key);
            match action {
                Action::Press => {
                    self.pressed.insert(*key);
                }
                Action::Release => {
                    self.pressed.remove(key);
                }
                Action::Repeat => (),
            }
            let previous_frame_key = self.previous_frame.get(&key);
            if action == Action::Press && previous_frame_key != Some(&Action::Press) {
                self.just_pressed.insert(*key);
            } else {
                self.just_pressed.remove(key);
            }
            if action == Action::Release && previous_frame_key != Some(&Action::Release) && previous_frame_key != None {
                self.just_released.insert(*key);
            } else {
                self.just_released.remove(key);
            }
            self.previous_frame.insert(*key, action);
        }

        if window.get_cursor_mode() == glfw::CursorMode::Normal {
            self.mouse_delta = (0.,0.);
            return;
        }

        let (mouse_x, mouse_y) = window.get_cursor_pos();

        let center_x = (crate::RES * 16 / 9 / 2) as f64;
        let center_y = (crate::RES / 2) as f64;

        let delta_x = mouse_x   - center_x;
        let delta_y = (mouse_y  - center_y) * -1.;


        if self.mouse_pos.is_none() {
            self.mouse_pos = Some((mouse_x,mouse_y));
        } else {
            self.mouse_delta = (delta_x,delta_y);
            self.mouse_pos = Some((mouse_x,mouse_y));
        }
    }
}

pub unsafe fn init(res: u32) -> (Glfw,PWindow, glfw::GlfwReceiver<(f64, glfw::WindowEvent)>) {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::DepthBits(Some(24)));
    let (mut window, events) = glfw.create_window(res * 16 / 9, res , "Glfw Triangle", glfw::WindowMode::Windowed).unwrap();

    window.make_current();
    glfw.set_swap_interval(SwapInterval::None);

    let w: *mut PRenderContext = &mut window.render_context() as *mut _ ;
    global_loader::load_global_gl(&|s| w.as_mut().unwrap().get_proc_address(ptr_to_str(s)) as *const _);

    glViewport(0, 0, (res * 16 / 9) as i32, res as i32);

    glClearColor(0.2, 0.3, 0.3, 1.0);
    glFrontFace(GL_CW);
    glEnable(GL_CULL_FACE);
    glEnable(GL_DEPTH_TEST);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

    (glfw,window,events)
}

pub unsafe fn ptr_to_str(ptr: *const u8) -> &'static str {
    ptr.is_null().then(|| panic!("Pointer is null"));
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let byte_slice = std::slice::from_raw_parts(ptr, len);
    std::str::from_utf8(byte_slice).expect("Invalid UTF-8")
}
pub struct TimeBuffer {
    pub index: usize,
    pub buffer: Vec<u128>,
    pub sum: u128,
}
impl TimeBuffer {
    pub fn new(len : usize) -> Self {
        Self {
            index: 0,
            buffer: vec![0;len],
            sum: 0,
        }
    }
    pub fn update(&mut self, next: u128) -> f64 {
        let curr = self.buffer[self.index];
        self.sum -= curr;
        self.sum += next;
        self.buffer[self.index] = next;
        self.index = (self.index + 1) % self.buffer.len();
        if curr == 0 {
            self.sum as f64 / self.index as f64
        } else {
            self.sum as f64 / self.buffer.len() as f64
        }
    }
}

static mut SEED: u32 = 123456789; // Change this seed as needed
pub fn simple_rng() -> f32 {
    unsafe {
        SEED = SEED.wrapping_mul(1664525).wrapping_add(1013904223); // LCG formula
        (SEED >> 8) as f32 / (1u32 << 24) as f32 // Scale to [0,1)
    }
}
//pub fn calculate_normals(p1:Vec3,p2:Vec3,p3:Vec3) -> Vec3 {
    //let edge12 = p2 - p1;
    //let edge13 = p3 - p1;
    //let norm = edge12.cross(edge13).norm();
    //norm
//}

pub const DIRECTIONS: [Vec3;6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

const ALL_KEYS: &[Key] = &[
    Key::Space ,
    Key::Apostrophe ,
    Key::Comma ,
    Key::Minus ,
    Key::Period ,
    Key::Slash ,
    Key::Num0 ,
    Key::Num1 ,
    Key::Num2 ,
    Key::Num3 ,
    Key::Num4 ,
    Key::Num5 ,
    Key::Num6 ,
    Key::Num7 ,
    Key::Num8 ,
    Key::Num9 ,
    Key::Semicolon ,
    Key::Equal ,
    Key::A ,
    Key::B ,
    Key::C ,
    Key::D ,
    Key::E ,
    Key::F ,
    Key::G ,
    Key::H ,
    Key::I ,
    Key::J ,
    Key::K ,
    Key::L ,
    Key::M ,
    Key::N ,
    Key::O ,
    Key::P ,
    Key::Q ,
    Key::R ,
    Key::S ,
    Key::T ,
    Key::U ,
    Key::V ,
    Key::W ,
    Key::X ,
    Key::Y ,
    Key::Z ,
    Key::LeftBracket ,
    Key::Backslash ,
    Key::RightBracket ,
    Key::GraveAccent ,
    Key::World1 ,
    Key::World2 ,
    Key::Escape ,
    Key::Enter ,
    Key::Tab ,
    Key::Backspace ,
    Key::Insert ,
    Key::Delete ,
    Key::Right ,
    Key::Left ,
    Key::Down ,
    Key::Up ,
    Key::PageUp ,
    Key::PageDown ,
    Key::Home ,
    Key::End ,
    Key::CapsLock ,
    Key::ScrollLock ,
    Key::NumLock ,
    Key::PrintScreen ,
    Key::Pause ,
    Key::F1 ,
    Key::F2 ,
    Key::F3 ,
    Key::F4 ,
    Key::F5 ,
    Key::F6 ,
    Key::F7 ,
    Key::F8 ,
    Key::F9 ,
    Key::F10 ,
    Key::F11 ,
    Key::F12 ,
    Key::F13 ,
    Key::F14 ,
    Key::F15 ,
    Key::F16 ,
    Key::F17 ,
    Key::F18 ,
    Key::F19 ,
    Key::F20 ,
    Key::F21 ,
    Key::F22 ,
    Key::F23 ,
    Key::F24 ,
    Key::F25 ,
    Key::Kp0 ,
    Key::Kp1 ,
    Key::Kp2 ,
    Key::Kp3 ,
    Key::Kp4 ,
    Key::Kp5 ,
    Key::Kp6 ,
    Key::Kp7 ,
    Key::Kp8 ,
    Key::Kp9 ,
    Key::KpDecimal ,
    Key::KpDivide ,
    Key::KpMultiply ,
    Key::KpSubtract ,
    Key::KpAdd ,
    Key::KpEnter ,
    Key::KpEqual ,
    Key::LeftShift ,
    Key::LeftControl ,
    Key::LeftAlt ,
    Key::LeftSuper ,
    Key::RightShift ,
    Key::RightControl ,
    Key::RightAlt ,
    Key::RightSuper ,
    Key::Menu ,
    //Key::Unknown ,
];
