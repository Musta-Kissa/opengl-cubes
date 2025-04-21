#![allow(dead_code)]
//#![allow(unused_parens)]
//#![allow(unused_variables)]
//
//#![allow(warnings)]

mod chunk;
mod mesh;
mod vertex;
mod utils;
mod shader;
mod camera;
mod octree;

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

mod ray;

pub const HEIGHT: u32 = 896;
pub const WIDTH: u32 = 1600;

pub const FPS: f64 = f64::MAX;

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

use utils::colors::*;

fn clear_screen() {
    use std::io::Write;
    print!("\x1b[2J\x1b[H");
    std::io::stdout().flush().unwrap();
}


fn main() {
    let (mut glfw, win, events) = unsafe { utils::init(WIDTH,HEIGHT) };
    let mut state = AppState::with_window(win);
    state.camera.pos= vec3!(-50.,50.,-50.);
    state.camera.dir = vec3!(1.,-1.,1.).norm();


    #[allow(unused_mut)]
    let mut chunk_data = chunk::gen_chunk_data();


    let mut chunk_brickmap = chunk::gen_brickmap();
    let mut brick_grid_ssbo = 0;
    let mut brick_data_ssbo = 0;

    let mut octree = chunk::gen_chunk_octree();
    //let mut octree = octree::Octree::new(1 << 2,ivec3!(0,0,0));
    //octree.remove_block(ivec3!(0,0,0));

    // Load shaders
    let (uv_passthrough_program,dda_program) = unsafe {
        use crate::shader::*;
        let uv_passthrough_vert         = compile_shader(gl::VERTEX_SHADER,"./shaders/uv_passthrough.vert");
        let texturig_frag               = compile_shader(gl::FRAGMENT_SHADER,"./shaders/texturing.frag");
        let dda_compute_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/dda_ray.comp");

        let uv_passthrough_program      = ShaderProgram::create_program(uv_passthrough_vert,texturig_frag);
        let dda_program                 = ShaderProgram::create_compute(dda_compute_shader);

        gl::DeleteShader(uv_passthrough_vert);
        gl::DeleteShader(texturig_frag);
        gl::DeleteShader(dda_compute_shader);
        (uv_passthrough_program,dda_program)
    };
    //panic!("{}GOOD PANIC{}",GREEN,RESET_COL);

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

    let texture = create_texture(WIDTH,HEIGHT);
    unsafe { gl::BindImageTexture(0, texture, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F) };
    #[allow(unused_mut)]
    let mut debug_data = vec![0f32;1024];
    let mut debug_ssbo = 0;
    let mut ssbo = 0;

    unsafe {
        use std::mem;
        use chunk::BRICK_GRID_SIZE;
        //use crate::octree::OctreeNode;

        //gl::GenBuffers(1, &mut ssbo);
        //gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
        //gl::BufferData(
            //gl::SHADER_STORAGE_BUFFER,
            //(octree.nodes.len() * mem::size_of::<OctreeNode>()) as isize,
            //octree.nodes.as_ptr() as *const _,
            //gl::DYNAMIC_DRAW,
        //);
        //gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, ssbo);
        
        gl::GenBuffers(1, &mut ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            (mem::size_of::<chunk::ChunkData>()) as isize,
            chunk_data.as_ptr() as *const _,
            gl::DYNAMIC_DRAW,
        );
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, ssbo);

        gl::GenBuffers(1, &mut debug_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, debug_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            (1024 * mem::size_of::<i32>()) as isize,
            debug_data.as_ptr() as *const _,
            gl::DYNAMIC_DRAW,
        );
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, debug_ssbo);

        // BRICK MAP SSBOs

        gl::GenBuffers(1, &mut brick_grid_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_grid_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            (mem::size_of::<[[[u32; BRICK_GRID_SIZE]; BRICK_GRID_SIZE]; BRICK_GRID_SIZE]>()) as isize,
            chunk_brickmap.brick_grid.as_ptr() as *const _,
            gl::DYNAMIC_DRAW,
        );
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, brick_grid_ssbo);

        gl::GenBuffers(1, &mut brick_data_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, brick_data_ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            (mem::size_of::<chunk::Brick>()) as isize * chunk_brickmap.brick_data.len() as isize,
            chunk_brickmap.brick_data.as_ptr() as *const _,
            gl::DYNAMIC_DRAW,
        );
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, brick_data_ssbo);
    }
    
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

        let camera = &state.camera;
        // RENDER /////////////////////////////////////////////////////////////////////////////////////////////////////////

        unsafe {
            gl::Clear(0);

            gl::UseProgram(*dda_program);
            dda_program.set_float("fov",camera.fov);
            dda_program.set_int("SIZE",chunk::SIZE as i32);
            dda_program.set_vec3("camera_pos",camera.pos);
            dda_program.set_vec3("camera_dir",camera.dir);
            dda_program.set_vec3("light_dir",state.light_dir);

            gl::DispatchCompute(WIDTH /16, 
                                HEIGHT/16, 1);

            //gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
            gl::MemoryBarrier(gl::ALL_BARRIER_BITS);
           
            //Draw texture
            gl::UseProgram(*uv_passthrough_program);
            screen_vao.draw_elements(gl::TRIANGLES);

            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, debug_ssbo);
            gl::GetBufferSubData(
                gl::SHADER_STORAGE_BUFFER,
                0,
                (debug_data.len() * std::mem::size_of::<i32>()) as _,
                debug_data.as_mut_ptr() as *mut _,
            );
            //println!("data len: {}",chunk_brickmap.brick_data.len());
            //println!("og     0: {}",chunk_brickmap.brick_grid[19][19][19]);
            //println!("{}debug 0: {}{}",MAGENTA,debug_data[0],RESET_COL);
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
                                if !octree.add_block(pos - hit_dir.into()) {
                                    break;
                                }
                                unsafe {
                                    use std::mem;
                                    use crate::octree::OctreeNode;
                                    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
                                    gl::BufferData(
                                        gl::SHADER_STORAGE_BUFFER,
                                        (octree.nodes.len() * mem::size_of::<OctreeNode>()) as isize,
                                        octree.nodes.as_ptr() as *const _,
                                        gl::DYNAMIC_DRAW,
                                    );
                                    gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, ssbo);
                                }
                            }
                        }
                        MouseButton::Button2 => {
                            if let Some((pos,_)) = ray::dda_3d_octree(camera.pos,camera.dir,100.,&octree) {
                                if !octree.remove_block(pos) {
                                    break;
                                }
                                unsafe {
                                    use std::mem;
                                    use crate::octree::OctreeNode;
                                    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
                                    gl::BufferData(
                                        gl::SHADER_STORAGE_BUFFER,
                                        (octree.nodes.len() * mem::size_of::<OctreeNode>()) as isize,
                                        octree.nodes.as_ptr() as *const _,
                                        gl::DYNAMIC_DRAW,
                                    );
                                    gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, ssbo);
                                }
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
        state.d_t = elapsed.as_nanos() as f32 / 1000_000. ; // in millis
        
        let avrg = time_buffer.update(elapsed.as_micros());
        state.window.set_title(&format!("{:.2}fps ({:.4?})",1./(avrg / 1000_000.),elapsed));
    }
}
