#![allow(dead_code)]
//#![allow(warnings)]

mod chunk;
mod mesh;
mod vertex;
mod utils;
mod shader;
mod camera;
mod octree;
mod octree_arr;

#[macro_use]
extern crate my_math;
use my_math::prelude::*;

//use gl33::*;
//use gl33::global_loader::*;

use glfw::{Context, Key, PWindow};
use std::{time,thread};
use std::time::Instant;
use crate::utils::*;
use crate::vertex::*;
use crate::mesh::Mesh;

//use gl::*;

use camera::Camera;

mod ray_arr;
//mod ray;
use ray_arr as ray;

pub const HEIGHT: u32 = 600;
pub const WIDTH: u32 = HEIGHT * 16/9;

pub const FPS: f64 = 60.;

struct AppState {
    window: PWindow,
    camera: Camera,
    d_t: f32,
    light_dir: Vec3,
    input: utils::InputTracker,

    wireframe: bool,
    cursor_enabled: bool,
    octree_skeleton: bool,
}
impl AppState {
    fn with_window(window: PWindow) -> Self {
        AppState {
            window,
            camera: Camera::default(),
            d_t: 1.,
            light_dir: vec3!(1.,0.,0.),
            input: utils::InputTracker::new(),

            wireframe: false,
            cursor_enabled:true,
            octree_skeleton: false,
        }
    }
}

const MAGENTA:&str = "\x1b[35m";
const GREEN:&str = "\x1b[32m";
const RESET_COL:&str = "\x1b[0m";

fn main() {
    let (mut glfw, win, events) = unsafe { utils::init(WIDTH,HEIGHT) };
    let mut state = AppState::with_window(win);
    
    let mut octree = chunk::gen_chunk_octree_2d();

    let mut octree_mesh = octree.gen_mesh();
    let mut octree_skeleton_mesh = octree.gen_skeleton_mesh();

    let mut octree_vao = unsafe { utils::vao_from_mesh(&octree_mesh) };
    let mut octree_skeleton_vao = unsafe { utils::vao_from_mesh(&octree_skeleton_mesh) };

    // Load shaders
    let (program,solid_color_program,solid_color_alpha_program,compute_program,uv_passthrough_program) = unsafe {
        use crate::shader::*;
        let perspective_vert            = compile_shader(gl::VERTEX_SHADER,"./shaders/3d_perspective.vert");
        let uv_passthrough_vert         = compile_shader(gl::VERTEX_SHADER,"./shaders/uv_passthrough.vert");
        let dir_light_frag              = compile_shader(gl::FRAGMENT_SHADER,"./shaders/dir_light.frag");
        let solid_color_frag            = compile_shader(gl::FRAGMENT_SHADER,"./shaders/pure_color.frag");
        let solid_color_alpha_frag      = compile_shader(gl::FRAGMENT_SHADER,"./shaders/pure_color_alpha.frag");
        let texturig_frag               = compile_shader(gl::FRAGMENT_SHADER,"./shaders/texturing.frag");
        let compute_shader              = compile_shader(gl::COMPUTE_SHADER,"./shaders/octree_ray.comp");

        let program                     = ShaderProgram::create_program(perspective_vert,dir_light_frag);
        let solid_color_program         = ShaderProgram::create_program(perspective_vert,solid_color_frag);
        let solid_color_alpha_program   = ShaderProgram::create_program(perspective_vert,solid_color_alpha_frag);
        let uv_passthrough_program      = ShaderProgram::create_program(uv_passthrough_vert,texturig_frag);
        let compute_program             = ShaderProgram::create_compute(compute_shader);

        gl::DeleteShader(perspective_vert);
        gl::DeleteShader(uv_passthrough_vert);
        gl::DeleteShader(dir_light_frag);
        gl::DeleteShader(solid_color_frag);
        gl::DeleteShader(solid_color_alpha_frag);
        gl::DeleteShader(compute_shader);
        (program,solid_color_program,solid_color_alpha_program,compute_program,uv_passthrough_program)
    };

    let mut screen_mesh = Mesh::new();
    screen_mesh.verts = vec![
        // Positions                            // Texture Coords
        UvVertex { pos: Vec2::new(-1.,-1.), uv_pos: Vec2::new( 0.0, 0.0) }, // Top-left;
        UvVertex { pos: Vec2::new(-1., 1.), uv_pos: Vec2::new( 0.0, 1.0) }, // Top-Right
        UvVertex { pos: Vec2::new( 1., 1.), uv_pos: Vec2::new( 1.0, 1.0) }, // Bottom-Right
        UvVertex { pos: Vec2::new( 1.,-1.), uv_pos: Vec2::new( 1.0, 0.0) }, // Bottom-Left
    ];
    screen_mesh.indices = vec![
        0, 1, 2, // First triangle
        2, 3, 0 // Second triangle
    ];
    let screen_vao = unsafe { vao_from_mesh(&screen_mesh) };

    let mut texture = create_texture(WIDTH,HEIGHT);
    unsafe { gl::BindImageTexture(0, texture, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F) };
    
    state.window.set_size_polling(true);
    state.window.set_key_polling(true);
    state.window.set_cursor_pos_polling(true);
    state.window.set_mouse_button_polling(true);

    let mut time_buffer = utils::TimeBuffer::new(40);

    while !state.window.should_close() {
        let frame_time = Instant::now();
        state.window.swap_buffers();

        state.input.update(&state.window);
        state.camera.update_with_input(&state.input,state.d_t);

        if state.window.get_cursor_mode() == glfw::CursorMode::Disabled {
            state.window.set_cursor_pos((WIDTH /2 ) as f64, (HEIGHT /2 ) as f64);
        }

        ////////////////////////////////////////////////////////////////////////////////////////////////////////////



        let camera = &state.camera;
        let cam_trans_mat = matrix::look_at_lh(camera.pos, camera.pos + camera.dir, camera.up);
        let proj = my_math::matrix::proj_mat_wgpu(camera.fov, 16. / 9., camera.near, camera.far);
        let view_proj = proj * cam_trans_mat;

        let mut collide_mesh = mesh::Mesh::new();

        let mut ghost_mesh: Mesh<Vertex> = Mesh::new();
        println!("cap left: {} size: {} Mib used size: {}",
                    octree.nodes.capacity() - octree.nodes.len(),
                    std::mem::size_of::<octree_arr::OctreeNode>()  as f32* octree.nodes.capacity()  as f32/ (1024 * 1024) as f32,
                    std::mem::size_of::<octree_arr::OctreeNode>()  as f32* octree.nodes.len()  as f32/ (1024 * 1024) as f32);
                    
        let start = Instant::now();
        for _ in 0..1000 {
            ray::ray_octree_dir(camera.pos,camera.dir,&octree);
        }
        println!("param ray time {:?}",start.elapsed()/1000);
        let start = Instant::now();
        for _ in 0..1000 {
            ray::dda_3d_octree(camera.pos,camera.dir,1000.,&octree);
        }
        println!("dda ray time {:?}",start.elapsed()/1000);

        if let Some((node,t)) = ray::ray_octree(camera.pos,camera.dir,&octree){
            let cube_color = vec3!(0.,0.,0.7);

            let hit = camera.pos + camera.dir * t;
            let hit_dir = ray::hit_direction(hit,camera.dir);

            //let ambient = 0.05;
            //let dot_light = state.light_dir.norm().dot((-hit_dir).into());
            //let ratio = (dot_light + 1.0) / 2.0;
            //let frag_color = ((cube_color * ratio) + vec3!(ambient,ambient,ambient));


            collide_mesh.join_with(&mesh::gen_icosahedron(0.10,hit,vec3!(1.,0.,1.)));

            //let hit_offset:IVec3 = hit_dir.into();
            //ghost_mesh.join_with(&mesh::gen_cube(node.size,(node.position - hit_offset * node.size).into(),vec3!(0.5,0.4,0.)));
        }

        let collide_vao = unsafe { utils::vao_from_mesh(&collide_mesh) };
        let ghost_vao = unsafe { utils::vao_from_mesh(&ghost_mesh) };


        // RENDER 
        unsafe {
            let transform_mat = view_proj.to_opengl();

            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::UseProgram(*program);
            program.set_mat4("transform",transform_mat.as_ptr() as *const _);
            program.set_vec3("light_dir",state.light_dir);

            gl::UseProgram(*solid_color_program);
            solid_color_program.set_mat4("transform",transform_mat.as_ptr() as *const _);

            gl::UseProgram(*solid_color_alpha_program);
            solid_color_alpha_program.set_mat4("transform",transform_mat.as_ptr() as *const _);


            gl::UseProgram(*program);
            octree_vao.draw_elements(gl::TRIANGLES);
            gl::UseProgram(*solid_color_program);
            collide_vao.draw_elements(gl::TRIANGLES);

            //chunk_vao.draw_elements(gl::TRIANGLES);

            gl::Enable(gl::BLEND);

            gl::UseProgram(*solid_color_alpha_program);
            //gl::DepthFunc(gl::ALWAYS);
            if state.octree_skeleton {
                octree_skeleton_vao.draw_elements(gl::LINES);
            }
            //gl::DepthFunc(gl::LESS);

            gl::UseProgram(*solid_color_alpha_program);
            ghost_vao.draw_elements(gl::TRIANGLES);
            gl::Disable(gl::BLEND);
            
            //gl::UseProgram(*compute_program);
            //gl::DispatchCompute(WIDTH /16, HEIGHT /16, 1);

            //gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
            //gl::MemoryBarrier(gl::ALL_BARRIER_BITS);

            // Draw texture
            //gl::UseProgram(*uv_passthrough_program);
            //screen_vao.draw_elements(gl::TRIANGLES);

        }

        glfw.poll_events();
        for (_ ,event) in glfw::flush_messages(&events) {
            use glfw::WindowEvent;
            use glfw::MouseButton;
            use glfw::Action;
            match event {
                WindowEvent::Size(x, y) => {
                    unsafe { gl::Viewport(0, 0, x, y); }
                }
                glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                    match button {
                        MouseButton::Button1 => {
                            if let Some((node,t)) = ray::ray_octree(camera.pos,camera.dir,&octree) {
                                let hit = camera.pos + camera.dir * t;
                                let mut pos = node.position;
                                if node.size != 1 {
                                    (pos,_) =  ray::dda_3d_octree(hit,camera.dir,node.size as f32 *2.,&octree).unwrap();
                                }
                                let hit_dir = ray::hit_direction(hit,camera.dir);
                                let start = std::time::Instant::now();
                                if !octree.add_block(pos - hit_dir.into()) {
                                    break;
                                }
                                //println!("regenerating skeleton mesh");
                                octree_skeleton_mesh = octree.gen_skeleton_mesh();
                                //println!("regenerating skeleton vao");
                                octree_skeleton_vao = unsafe { utils::vao_from_mesh(&octree_skeleton_mesh) };
                                //println!("regenerating octree mesh");
                                octree_mesh = octree.gen_mesh();
                                //println!("regenerating octree vao");
                                octree_vao = unsafe { utils::vao_from_mesh(&octree_mesh) };
                                //println!("finnished");
                            }
                        }
                        MouseButton::Button2 => {
                            if let Some((pos,_)) = ray::dda_3d_octree(camera.pos,camera.dir,100.,&octree) {
                                if !octree.remove_block(pos) {
                                    break;
                                }

                                //println!("regenerating skeleton mesh");
                                octree_skeleton_mesh = octree.gen_skeleton_mesh();
                                //println!("regenerating skeleton vao");
                                octree_skeleton_vao = unsafe { utils::vao_from_mesh(&octree_skeleton_mesh) };
                                //println!("regenerating octree mesh");
                                octree_mesh = octree.gen_mesh();
                                //println!("regenerating octree vao");
                                octree_vao = unsafe { utils::vao_from_mesh(&octree_mesh) };
                                //println!("finnished");
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
                Key::B => {
                    state.octree_skeleton = !state.octree_skeleton;
                }
                Key::Y => {
                    state.wireframe = !state.wireframe;
                    unsafe { 
                        if state.wireframe {
                            gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE); 
                            gl::Disable(gl::CULL_FACE);
                        } else {
                            gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL); 
                            gl::Enable(gl::CULL_FACE);
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
                    frame_time.elapsed()
                )
        );

        let elapsed = frame_time.elapsed();
        state.d_t = elapsed.as_micros() as f32 / 1000. ; // in millis
        
        let avrg = time_buffer.update(elapsed.as_micros());
        state.window.set_title(&format!("{:.2}fps ({:?})",1./(avrg / 1000_000.),elapsed));
    }
}

