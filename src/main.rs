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

use glfw::{Context, Key, PWindow};
use std::{time,thread};
use std::time::{Instant,Duration};
use std::sync::mpsc;
use crate::utils::*;
use crate::vertex::*;
use crate::mesh::Mesh;

use camera::Camera;

pub const HEIGHT: u32 = 1000;
pub const WIDTH: u32 = HEIGHT * 16/9;

pub const FPS: f64 = f64::MAX;
pub const CHUNK_RADIUS: f32 = 3.5;

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
            light_dir: vec3!(1.,1.,0.).norm(),
            input: utils::InputTracker::new(),

            wireframe: false,
            cursor_enabled:true,
            octree_skeleton: false,
        }
    }
}

fn clear_screen() {
    use std::io::Write;
    print!("\x1b[2J\x1b[H");
    std::io::stdout().flush().unwrap();
}

fn main() {
    let (mut glfw, win, events) = unsafe { utils::init(WIDTH,HEIGHT) };
    
    let (mut shared_window, _) = win
        .create_shared(1, 1, "Shared Context", glfw::WindowMode::Windowed)
        .expect("Failed to create shared context");
    shared_window.hide();

    let mut state = AppState::with_window(win);
    //state.camera.pos= vec3!(1900./3.5+ 256.0,
                            //256 as f32 *  1.5,
                            //1900./3.5+ 512.0);
    state.camera.pos = Vec3 { x: chunk::SIZE as f32 / 2., y: 1320.88193, z: chunk::SIZE as f32 / 2.};
    state.camera.dir = vec3!(-1.,-1.,-1.).norm();


    // Load shaders
    let (screen_texturing_program,dda_program,clear_texture) = unsafe {
        use crate::shader::*;
        let uv_passthrough_vert         = compile_shader(gl::VERTEX_SHADER,"./shaders/uv_passthrough.vert");
        let texturig_frag               = compile_shader(gl::FRAGMENT_SHADER,"./shaders/texturing.frag");
        //let dda_compute_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/dda_ray.comp");
        let dda_compute_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/dda_brick.comp");
        let clear_texture_shader        = compile_shader(gl::COMPUTE_SHADER,"./shaders/clear_texture.comp");

        let screen_texturing_program    = ShaderProgram::create_program(uv_passthrough_vert,texturig_frag);
        let dda_program                 = ShaderProgram::create_compute(dda_compute_shader);
        let clear_texture               = ShaderProgram::create_compute(clear_texture_shader);

        gl::DeleteShader(uv_passthrough_vert);
        gl::DeleteShader(texturig_frag);
        gl::DeleteShader(dda_compute_shader);
        gl::DeleteShader(clear_texture_shader);
        (screen_texturing_program,dda_program,clear_texture)
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

    let texture = create_texture(WIDTH,HEIGHT);
    unsafe { gl::BindImageTexture(0, texture, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F) };

    state.window.set_size_polling(true);
    state.window.set_key_polling(true);
    state.window.set_cursor_pos_polling(true);
    state.window.set_mouse_button_polling(true);

    let mut time_buffer = utils::TimeBuffer::new(40);

    let (out_tx, out_rx) = mpsc::channel();
    let (request_tx, request_rx) = mpsc::channel();

    let mut target_chunks = gen_pos_in_radius(state.camera.pos) ;
    for pos in &target_chunks {
        request_tx.send(*pos).unwrap();
    }
    
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    let generate_thread_stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&generate_thread_stop_flag);
    let generate_thread = thread::spawn(move || {
        use crate::chunk::Chunk;
        shared_window.make_current();
        gl::load_with(|s| shared_window.get_proc_address(s) as *const _);
        while !stop_flag.load(Ordering::Relaxed) {
            if let Ok(pos) = request_rx.recv_timeout(Duration::from_millis(10)) {
                let brickmap = chunk::gen_brickmap_2d(pos);
                let (brickmap_grid_ssbo, brickmap_data_ssbo) = unsafe { brickmap.gen_ssbos() };

                unsafe { gl::Flush() }; // Finish sending data to ssbo's
                out_tx.send( Chunk { brickmap, brickmap_data_ssbo, brickmap_grid_ssbo, pos } ).unwrap();
            }
        }
    });

    let mut chunks: Vec<chunk::Chunk> = Vec::new();

    while !state.window.should_close() {
        let frame_time = Instant::now();
        
        state.input.update(&state.window);
        state.camera.update_with_input(&state.input,state.d_t);
        if state.window.get_cursor_mode() == glfw::CursorMode::Disabled {
            state.window.set_cursor_pos((WIDTH /2 ) as f64, (HEIGHT /2 ) as f64);
        }
        let camera = &state.camera;

        match out_rx.try_recv() {
            Ok(chunk) => {
                chunks.push(chunk);
            },
            _ => (),
        }
        println!("CHUNK NUMBER: {} TARGER: {}",chunks.len(),target_chunks.len());

        // UPDATE CHUNKS
        {
            let camera_pos = camera.pos / chunk::SIZE as f32;
            let r_squared = CHUNK_RADIUS*CHUNK_RADIUS;

            //target_chunks.retain(|pos| {
                //let dx = pos.x as f32 + 0.5 - camera_pos.x;
                //let dz = pos.z as f32 + 0.5 - camera_pos.z;
                //(dx*dx + dz*dz) <= r_squared // CHUNK POS IS STILL VALID
            //});

            // REMOVE CHUNKS
            let mut i = 0;
            while i < chunks.len() {
                let pos = chunks[i].pos;
                let dx = pos.x as f32 + 0.5 - camera_pos.x;
                let dz = pos.z as f32 + 0.5 - camera_pos.z;
                if (dx*dx + dz*dz) <= r_squared { // CHUNK POS IS STILL VALID
                    i+=1;
                } else { // REMOVE CHUNK
                    unsafe {
                    gl::DeleteBuffers(1, &chunks[i].brickmap_grid_ssbo);
                    gl::DeleteBuffers(1, &chunks[i].brickmap_data_ssbo);
                    }
                    //remove_by_value(&mut target_chunks,&chunks[i].pos);
                    chunks.swap_remove(i);
                }
            }
            target_chunks.retain(|pos| {
                let dx = pos.x as f32 + 0.5 - camera_pos.x;
                let dz = pos.z as f32 + 0.5 - camera_pos.z;
                (dx*dx + dz*dz) <= r_squared // CHUNK POS IS STILL VALID
            });

            // ADD CHUNKS
            let min_x = (camera_pos.x - CHUNK_RADIUS).floor() as i32;
            let min_z = (camera_pos.z - CHUNK_RADIUS).floor() as i32;
            let max_x = (camera_pos.x + CHUNK_RADIUS).ceil() as i32;
            let max_z = (camera_pos.z + CHUNK_RADIUS).ceil() as i32;

            let mut pos_to_add = Vec::new();
            for x in min_x..=max_x {
                for z in min_z..=max_z {
                    let dx = x as f32 + 0.5 - camera_pos.x;
                    let dz = z as f32 + 0.5 - camera_pos.z;
                    if (dx*dx + dz*dz) <= r_squared && !target_chunks.contains(&ivec3!(x,0,z)) {
                        println!("adding {} {} {}",x,0,z);
                        pos_to_add.push(ivec3!(x,0,z));
                    }
                }
            }
            let dist_to_camera = |pos: IVec3| {
                ((pos.as_vec3() + 0.5) - camera_pos).mag()
            };
            pos_to_add.sort_by(|a,b| {
                dist_to_camera(*a).partial_cmp(&dist_to_camera(*b))
                .expect("Coundnt compare")
            });
            for pos in pos_to_add {
                target_chunks.push(pos);
                request_tx.send(pos);
            }
        }
        
        let dist_to_camera = |pos: IVec3| {
            (camera.pos - (pos * chunk::SIZE as i32 + (chunk::SIZE as i32/2)).as_vec3() ).mag()
        };
        chunks.sort_by(|a,b| {
            dist_to_camera(a.pos).partial_cmp(&dist_to_camera(b.pos))
            .expect("Coundnt compare")
        });

        // RENDER /////////////////////////////////////////////////////////////////////////////////////////////////////////

        unsafe {
            gl::UseProgram(*dda_program);
            dda_program.set_float("fov",camera.fov);
            dda_program.set_int("SIZE",chunk::SIZE as i32);
            dda_program.set_vec3("camera_pos",camera.pos);
            dda_program.set_vec3("camera_dir",camera.dir);
            dda_program.set_vec3("light_dir",state.light_dir);
            
            // Color texture
            for chunk in &chunks {
                dda_program.set_ivec3("pos",chunk.pos);

                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, chunk.brickmap_grid_ssbo);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, chunk.brickmap_data_ssbo);

                gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
            }
            //gl::MemoryBarrier(gl::ALL_BARRIER_BITS);
            
            // Draw texture
            gl::UseProgram(*screen_texturing_program);
            screen_vao.draw_elements(gl::TRIANGLES);
            // Clear texture
            gl::UseProgram(*clear_texture);
            gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
            //gl::MemoryBarrier(gl::ALL_BARRIER_BITS);
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
                /*
                glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                    match button {
                        MouseButton::Button1 => {
                        MouseButton::Button2 => {
                        _ => (),
                    }                
                }
                */
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
        let test_time = time::Instant::now();
        state.window.swap_buffers();

        if test_time.elapsed() > Duration::from_millis(17) {
            println!("buffer swap time: {:?}",test_time.elapsed());
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
        let fps_string = format!("{:.2}fps ({:.4?})",1./(avrg / 1000_000.),elapsed);
        state.window.set_title(&fps_string);
    }

    generate_thread_stop_flag.store(true, Ordering::Relaxed);
    generate_thread.join();
}
fn gen_chunk_pos(size: i32) -> Vec<IVec3> {
    let mut out = Vec::new();

    for x in 0..size {
        for y in 0..size {
            out.push(ivec3!(x,0,y));
        }
    }
    out
}
fn gen_pos_in_radius(camera_pos: Vec3) -> Vec<IVec3> {
    let camera_pos = camera_pos / chunk::SIZE as f32;
    let mut positions = Vec::new();
    let r_squared = CHUNK_RADIUS*CHUNK_RADIUS;

    let min_x = (camera_pos.x - CHUNK_RADIUS).floor() as i32;
    let max_x = (camera_pos.x + CHUNK_RADIUS).ceil() as i32;
    let min_z = (camera_pos.z - CHUNK_RADIUS).floor() as i32;
    let max_z = (camera_pos.z + CHUNK_RADIUS).ceil() as i32;

    for x in min_x..=max_x {
        for z in min_z..=max_z {
            let dx = x as f32 + 0.5 - camera_pos.x;
            let dz = z as f32 + 0.5 - camera_pos.z;
            if (dx*dx + dz*dz) <= r_squared {
                positions.push(ivec3!(x,0,z));
            }
        }
    }
    let dist_to_camera = |pos: IVec3| {
        ((pos.as_vec3() + 0.5) - camera_pos).mag()
    };
    positions.sort_by(|a,b| {
        dist_to_camera(*a).partial_cmp(&dist_to_camera(*b))
        .expect("Coundnt compare")
    });
    positions
}

fn remove_by_value<T: std::cmp::PartialEq<T>>(vec: &mut Vec<T>, value: &T) {
    if let Some(index) = vec.iter().position(|x| *x == *value) {
        vec.remove(index);
    }
}
