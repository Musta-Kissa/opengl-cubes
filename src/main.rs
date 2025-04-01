#![allow(dead_code)]
//#![allow(warnings)]

mod chunk;
mod mesh;
mod vertex;
mod utils;
mod shader;
mod ray;
mod camera;
mod octree;

#[macro_use]
extern crate my_math;
use my_math::prelude::*;

use gl33::*;
use gl33::global_loader::*;

use glfw::{Context, Key, PWindow};
use std::{time,thread};
use std::time::Instant;

use utils::Vao;
use camera::Camera;


pub const RES: u32 = 600;
pub const FPS: f64 = 60.;

struct AppState {
    window: PWindow,
    wireframe: bool,
    cursor_enabled: bool,
    camera: Camera,
    d_t: f32,
    light_dir: Vec3,
    input: utils::InputTracker,
}
impl AppState {
    fn with_window(window: PWindow) -> Self {
        AppState {
            window,
            wireframe: false,
            cursor_enabled:true,
            camera: Camera::default(),
            d_t: 1.,
            light_dir: vec3!(1.,0.,0.),
            input: utils::InputTracker::new(),
        }
    }
}

const MAGENTA:&str = "\x1b[35m";
const GREEN:&str = "\x1b[32m";
const RESET_COL:&str = "\x1b[0m";

#[derive(Debug,Copy,Clone)]
pub enum Dir {
    X,
    NEG_X,
    Y,
    NEG_Y,
    Z,
    NEG_Z,
}
impl Into<Vec3> for Dir {
    fn into(self) -> Vec3 {
        match self {
            Dir::X => Vec3::X,
            Dir::NEG_X => Vec3::NEG_X,
            Dir::Y => Vec3::Y,
            Dir::NEG_Y => Vec3::NEG_Y,
            Dir::Z => Vec3::Z,
            Dir::NEG_Z => Vec3::NEG_Z,
        }
    }
}
impl Into<IVec3> for Dir {
    fn into(self) -> IVec3 {
        match self {
            Dir::X => IVec3::X,
            Dir::NEG_X => IVec3::NEG_X,
            Dir::Y => IVec3::Y,
            Dir::NEG_Y => IVec3::NEG_Y,
            Dir::Z => IVec3::Z,
            Dir::NEG_Z => IVec3::NEG_Z,
        }
    }
}


fn hit_direction(hit: Vec3, dir: Vec3) -> Dir {
    let x = (hit.x - hit.x.round()).abs();
    let y = (hit.y - hit.y.round()).abs();
    let z = (hit.z - hit.z.round()).abs();

    if x < y && x < z {
        if dir.x < 0. {
            return Dir::NEG_X;
        } else {
            return Dir::X;
        }
    } else if y < z {
        if dir.y < 0. {
            return Dir::NEG_Y;
        } else {
            return Dir::Y;
        }
    } else {
        if dir.z < 0. {
            return Dir::NEG_Z;
        } else {
            return Dir::Z;
        }
    }
}

fn main() {
    let (mut glfw, win, events) = unsafe { utils::init(RES) };
    let mut state = AppState::with_window(win);
    let camera = state.camera;
    
    let mut octree = octree::Octree::new(16,ivec3!(0,0,0));
    octree.add_block(ivec3!(0,0,0));
    octree.add_block(ivec3!(0,0,1));
    octree.add_block(ivec3!(0,1,0));
    octree.add_block(ivec3!(0,1,1));
    octree.add_block(ivec3!(1,0,0));
    octree.add_block(ivec3!(1,0,1));
    octree.add_block(ivec3!(1,1,0));
    octree.add_block(ivec3!(1,1,1));

    let mut octree_skeleton_mesh = octree.gen_skeleton_mesh();
    let mut octree_skeleton_vao = unsafe { utils::vao_from_mesh(&octree_skeleton_mesh) };
    let mut octree_mesh = octree.gen_mesh();
    let mut octree_vao = unsafe { utils::vao_from_mesh(&octree_mesh) };

    let chunk_data = chunk::gen_chunk_data();

    let mut chunk_mesh = chunk::gen_mesh(&chunk_data);

    let chunk_vao = unsafe { utils::vao_from_mesh(&chunk_mesh) };

    // Load shaders
    let (program,solid_color_program,solid_color_alpha_program) = unsafe {
    use crate::shader::*;
        let vertex_shader   = compile_shader(GL_VERTEX_SHADER,"./shaders/vertex_shader.glsl");
        let fragment_shader = compile_shader(GL_FRAGMENT_SHADER,"./shaders/fragment_shader.glsl");
        let solid_color_fragment = compile_shader(GL_FRAGMENT_SHADER,"./shaders/pure_color_fr.glsl");
        let solid_color_alpha_fragment = compile_shader(GL_FRAGMENT_SHADER,"./shaders/pure_color_alpha_fr.glsl");
        let program         = ShaderProgram::create_program(vertex_shader,fragment_shader);
        let solid_color_program = ShaderProgram::create_program(vertex_shader,solid_color_fragment);
        let solid_color_alpha_program = ShaderProgram::create_program(vertex_shader,solid_color_alpha_fragment);
        glDeleteShader(vertex_shader);
        glDeleteShader(fragment_shader);
        glDeleteShader(solid_color_fragment);
        glDeleteShader(solid_color_alpha_fragment);
        (program,solid_color_program,solid_color_alpha_program)
    };
    
    state.window.set_size_polling(true);
    state.window.set_key_polling(true);
    state.window.set_cursor_pos_polling(true);
    state.window.set_mouse_button_polling(true);

    let mut time_buffer = utils::TimeBuffer::new(40);

    while !state.window.should_close() {
        let start = Instant::now();
        state.window.swap_buffers();

        state.input.update(&state.window);
        state.camera.update_with_input(&state.input,state.d_t);

        if state.window.get_cursor_mode() == glfw::CursorMode::Disabled {
            state.window.set_cursor_pos((RES /2 * 16/9) as f64, (RES /2 ) as f64);
        }

        ////////////////////////////////////////////////////////////////////////////////////////////////////////////



        let camera = &state.camera;
        let cam_trans_mat = matrix::look_at_lh(camera.pos, camera.pos + camera.dir, camera.up);
        let proj = my_math::matrix::proj_mat_wgpu(camera.fov, 16. / 9., camera.near, camera.far);
        let view_proj = proj * cam_trans_mat;

        let mut collide_mesh = mesh::Mesh::new();
        let mut ghost_mesh = mesh::Mesh::new();
        //if let Some((block,hit)) = ray::dda_3d(camera.pos,camera.dir, 1000.,&chunk_data) {
            //collide_mesh.join_with(&mesh::gen_cube(1,block.into()));
            //collide_mesh.join_with(&mesh::gen_icosahedron(0.10,hit,vec3!(1.,0.,1.)));
        //}
        if let Some((node_pos,hit)) = ray::dda_3d_octree(camera.pos,camera.dir,100.,&octree) {
            let hit_dir = hit_direction(hit,camera.dir);
            collide_mesh.join_with(&mesh::gen_icosahedron(0.10,hit,vec3!(1.,0.,1.)));
            ghost_mesh.join_with(&mesh::gen_cube(1,(node_pos - hit_dir.into() ).into(),vec3!(0.5,0.4,0.)));
            //println!("dir: {:?}",hit_dir);
        }

        let collide_vao = unsafe { utils::vao_from_mesh(&collide_mesh) };
        let ghost_vao = unsafe { utils::vao_from_mesh(&ghost_mesh) };


        // RENDER 
        unsafe {
            let transform_mat = view_proj.to_opengl();

            glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
            glUseProgram(*program);
            program.set_mat4("transform",transform_mat.as_ptr() as *const _);
            program.set_vec3("light_dir",state.light_dir);
            glUseProgram(*solid_color_program);
            solid_color_program.set_mat4("transform",transform_mat.as_ptr() as *const _);
            glUseProgram(*solid_color_alpha_program);
            solid_color_alpha_program.set_mat4("transform",transform_mat.as_ptr() as *const _);


            glUseProgram(*program);
            octree_vao.draw_elements(GL_TRIANGLES);
            glUseProgram(*solid_color_program);
            collide_vao.draw_elements(GL_TRIANGLES);

            //chunk_vao.draw_elements(GL_TRIANGLES);

            glEnable(GL_BLEND);

            glUseProgram(*solid_color_alpha_program);
            glDepthFunc(GL_ALWAYS);
            //octree_skeleton_vao.draw_elements(GL_LINES);
            glDepthFunc(GL_LESS);

            glUseProgram(*solid_color_alpha_program);
            ghost_vao.draw_elements(GL_TRIANGLES);
            glDisable(GL_BLEND);

        }

        glfw.poll_events();
        for (_ ,event) in glfw::flush_messages(&events) {
            use glfw::WindowEvent;
            use glfw::MouseButton;
            use glfw::Action;
            match event {
                WindowEvent::Size(x, y) => {
                    unsafe { glViewport(0, 0, x, y); }
                }
                glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                    match button {
                        MouseButton::Button1 => {
                            if let Some((pos,hit)) = ray::dda_3d_octree(camera.pos,camera.dir,100.,&octree) {
                                let hit_dir = hit_direction(hit,camera.dir);
                                octree.add_block(pos - hit_dir.into());

                                octree_skeleton_mesh = octree.gen_skeleton_mesh();
                                octree_skeleton_vao = unsafe { utils::vao_from_mesh(&octree_skeleton_mesh) };
                                octree_mesh = octree.gen_mesh();
                                octree_vao = unsafe { utils::vao_from_mesh(&octree_mesh) };
                            }
                        }
                        MouseButton::Button2 => {
                            if let Some((pos,_)) = ray::dda_3d_octree(camera.pos,camera.dir,100.,&octree) {
                                octree.remove_block(pos);

                                octree_skeleton_mesh = octree.gen_skeleton_mesh();
                                octree_skeleton_vao = unsafe { utils::vao_from_mesh(&octree_skeleton_mesh) };
                                octree_mesh = octree.gen_mesh();
                                octree_vao = unsafe { utils::vao_from_mesh(&octree_mesh) };
                            }
                        }
                        _ => (),
                    }                
                }
                _ => (),
            }
        }
        for key in &state.input.just_pressed {
            match key {
                Key::GraveAccent => {
                    state.cursor_enabled = !state.cursor_enabled;
                    if state.cursor_enabled {
                        state.window.set_cursor_mode(glfw::CursorMode::Normal);
                    } else {
                        state.window.set_cursor_mode(glfw::CursorMode::Disabled);
                    }
                }
                Key::Y => {
                    state.wireframe = !state.wireframe;
                    unsafe { 
                        if state.wireframe {
                            glPolygonMode(GL_FRONT_AND_BACK, GL_LINE); 
                            glDisable(GL_CULL_FACE);
                        } else {
                            glPolygonMode(GL_FRONT_AND_BACK, GL_FILL); 
                            glEnable(GL_CULL_FACE);
                        }
                    }
                }
                _ => (),
            }
        }
        for key in &state.input.pressed {
            match key {
                Key::Escape => state.window.set_should_close(true),

                Key::H => state.light_dir.rot_quat(1. * state.d_t / 16. ,vec3!(-1.,0.,1.)),

                _ => (),
            }
        }

        /////////////////////////////////////////////////////////////////////////////////////////////////////////
        thread::sleep(
                time::Duration::from_micros(
                    (1./FPS * 1e6 as f64).round() as u64
                ).saturating_sub(
                    start.elapsed()
                )
        );

        let elapsed = start.elapsed();
        state.d_t = elapsed.as_micros() as f32 / 1000. ; // in millis
        
        let avrg = time_buffer.update(elapsed.as_micros());
        state.window.set_title(&format!("{:.2}fps ({:?})",1./(avrg / 1000_000.),elapsed));
    }
}

