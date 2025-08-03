#![allow(dead_code)]
#![allow(non_upper_case_globals)]
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
mod entity;
mod brickmap;
#[macro_use]
extern crate my_math;

use my_math::prelude::*;

use glfw::{Context, Key, PWindow};
use std::{time,thread::{self,JoinHandle}};
use std::time::{Instant,Duration};
use std::sync::mpsc;
use crate::utils::*;
use crate::vertex::*;
use crate::mesh::Mesh;
use crate::chunk::CHUNK_SIZE;
use crate::entity::{Entity,GPUEntity};
use std::sync::{Arc,Mutex, atomic::{AtomicBool, Ordering}};
use crate::brickmap::Brick;

use camera::Camera;

pub const HEIGHT: u32 = 1000;
pub const WIDTH: u32 = HEIGHT * 16/9;

pub const FPS: f64 = f64::MAX;
pub const CHUNK_RADIUS: f32 = 1.5;
pub const GENERATOR_THREAD_COUNT: u32 = 2;

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
            light_dir: Vec3 { x: 0.98123676, y: -0.05549547, z: 0.2620155 }.norm(),
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
    

    let mut state = AppState::with_window(win);
    //state.camera.pos= vec3!(1900./3.5+ 256.0,
                            //256 as f32 *  1.5,
                            //1900./3.5+ 512.0);
    //state.camera.pos = Vec3 { x: chunk::SIZE as f32 / 2., y: 220.88193, z: chunk::SIZE as f32 / 2.};
    state.camera.pos = vec3!(15.,313.,12.);
    state.camera.pos = Vec3 { x: -11.250015, y: 322.1623, z: 24.850925 };
    state.camera.dir = Vec3 { x: 0.9999848, y: -0.0052359644, z: -0.0017453047 };
    state.camera.speed = 64.;


    // Load shaders
    let (screen_texturing_program,clear_texture,draw_entity_program) = unsafe {
        use crate::shader::*;
        let uv_passthrough_vert         = compile_shader(gl::VERTEX_SHADER,"./shaders/uv_passthrough.vert");
        let texturig_frag               = compile_shader(gl::FRAGMENT_SHADER,"./shaders/texturing.frag");
        //let dda_compute_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/dda_ray.comp");
        //let dda_compute_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/dda_brick.comp");
        let clear_texture_shader        = compile_shader(gl::COMPUTE_SHADER,"./shaders/clear_texture.comp");
        let draw_entity_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/draw_entity.comp");

        let screen_texturing_program    = ShaderProgram::create_program(uv_passthrough_vert,texturig_frag);
        //let dda_program                 = ShaderProgram::create_compute(dda_compute_shader);
        let clear_texture               = ShaderProgram::create_compute(clear_texture_shader);
        let draw_entity_program         = ShaderProgram::create_compute(draw_entity_shader);

        gl::DeleteShader(uv_passthrough_vert);
        gl::DeleteShader(texturig_frag);
        //gl::DeleteShader(dda_compute_shader);
        gl::DeleteShader(clear_texture_shader);
        (screen_texturing_program,clear_texture,draw_entity_program)
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
    let request_rx = Arc::new(Mutex::new(request_rx));

    let mut target_chunks = gen_pos_in_radius(state.camera.pos) ;
    for pos in &target_chunks {
        request_tx.send(*pos).unwrap();
    }
    

    let generate_thread_stop_flag = Arc::new(AtomicBool::new(false));
    let generate_thread_handles:Vec<JoinHandle<()>> = 
        (0..GENERATOR_THREAD_COUNT).map(|_| 
            spawn_generator_thread(
                &state.window,
                Arc::clone(&request_rx),
                Arc::clone(&generate_thread_stop_flag),
                out_tx.clone(),
        )).collect();

    let mut chunks: Vec<chunk::Chunk> = Vec::new();
    let mut test_entity = entity::gen_test_entity();



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

        // UPDATE CHUNKS
        {
            let mut change_flag = false;
            let camera_pos = camera.pos / chunk::CHUNK_SIZE as f32;
            let r_squared = CHUNK_RADIUS*CHUNK_RADIUS;

            //target_chunks.retain(|pos| {
                //let dx = pos.x as f32 + 0.5 - camera_pos.x;
                //let dz = pos.z as f32 + 0.5 - camera_pos.z;
                //(dx*dx + dz*dz) <= r_squared // CHUNK POS IS STILL VALID
            //});

            // REMOVE CHUNKS
            let mut i = 0;
            while i < chunks.len() {
                let pos = chunks[i].gpu_entity.pos / chunk::CHUNK_SIZE as f32;
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
                    change_flag = true;
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
                        pos_to_add.push(ivec3!(x,0,z));
                        change_flag = true;
                    }
                }
            }
            let dist_to_camera = |pos: IVec3| {
                ((pos.as_vec3() + 0.5)*chunk::CHUNK_SIZE as f32 - camera_pos).mag()
            };
            pos_to_add.sort_by(|a,b| {
                dist_to_camera(*a).partial_cmp(&dist_to_camera(*b))
                .expect("Coundnt compare")
            });
            for pos in pos_to_add {
                target_chunks.push(pos);
                let _ = request_tx.send(pos);
            }
            if change_flag {
                println!("CHUNK NUMBER: {} TARGER: {}",chunks.len(),target_chunks.len());
            }
        }
        
        let dist_to_camera = |pos: Vec3| {
            //(camera.pos - (pos * chunk::CHUNK_SIZE as f32 + (chunk::CHUNK_SIZE as i32/2) as f32) ).mag()
            (camera.pos - (pos + (chunk::CHUNK_SIZE as i32/2) as f32) ).mag()
        };
        chunks.sort_by(|a,b| {
            dist_to_camera(a.gpu_entity.pos).partial_cmp(&dist_to_camera(b.gpu_entity.pos))
            .expect("Coundnt compare")
        });

        use crate::entity::GPUEntity;
        let mut gpu_entities = vec![test_entity.gpu_entity];

        for chunk in &chunks {
            gpu_entities.push(chunk.gpu_entity);
        }

        let mut gpu_entities_ssbo = 0;
        unsafe {
            gl::GenBuffers(1, &mut gpu_entities_ssbo);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, gpu_entities_ssbo);
            gl::BufferData(
                gl::SHADER_STORAGE_BUFFER,
                (gpu_entities.len() * core::mem::size_of::<GPUEntity>()) as isize,
                gpu_entities.as_ptr() as *const _,
                gl::DYNAMIC_DRAW,
            );
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
        }
        // debug ssbo 
        let mut debug_ssbo: u32 = 0;
        unsafe {
            gl::GenBuffers(1, &mut debug_ssbo);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, debug_ssbo);

            let init_vels = [crate::brickmap::new_brick();16];
            let buffer_size = (8 * std::mem::size_of::<Brick>()) as isize;
            gl::BufferData(gl::SHADER_STORAGE_BUFFER, buffer_size, init_vels.as_ptr() as *const _, gl::DYNAMIC_COPY);

            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
        }

        // RENDER /////////////////////////////////////////////////////////////////////////////////////////////////////////

        unsafe {
            // Color texture
            gl::UseProgram(*draw_entity_program);

            let (local_ray_pos,local_ray_dir) = entity::ray_to_local(&test_entity,camera.pos,camera.dir);
            draw_entity_program.set_vec3("camera_pos",local_ray_pos);
            draw_entity_program.set_vec3("camera_dir",local_ray_dir);

            draw_entity_program.set_float("fov",camera.fov);
            draw_entity_program.set_vec3("light_dir",state.light_dir);
            draw_entity_program.set_uint("ENTITY_NUM",gpu_entities.len() as u32);

            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, gpu_entities_ssbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, test_entity.brickmap_grid_ssbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, test_entity.brickmap_data_ssbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 4, debug_ssbo);

            gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);


            /*
            for chunk in &chunks {
                let (local_ray_pos,local_ray_dir) = entity::ray_to_local(&chunk,camera.pos,camera.dir);
                draw_entity_program.set_vec3("camera_pos",local_ray_pos);
                draw_entity_program.set_vec3("camera_dir",local_ray_dir);
                //dda_program.set_ivec3("CHUNK_POS",chunk.pos);

                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, chunk.brickmap_grid_ssbo);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, chunk.brickmap_data_ssbo);

                gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
            }
            */

            {
                let mut debug_data = [crate::brickmap::new_brick();16];
                unsafe {
                    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, debug_ssbo);

                    let ptr = gl::MapBuffer(gl::SHADER_STORAGE_BUFFER, gl::READ_ONLY) as *const Brick;
                    if !ptr.is_null() {
                        // Copy into your own buffer
                        std::ptr::copy_nonoverlapping(ptr, debug_data.as_mut_ptr(), debug_data.len());
                        gl::UnmapBuffer(gl::SHADER_STORAGE_BUFFER);
                    }

                    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
                }
                //for n in 0..8 {
                    ////println!("{:?} \n -> \n {:?}", debug_data[n*2],debug_data[n*2+1]);
                    //if( debug_data[n*2] != debug_data[n*2+1]) {
                       //panic!(); 
                    //} else {
                        //println!("same");
                    //}
                //}
            }

            
            // Draw texture
            gl::UseProgram(*screen_texturing_program);
            screen_vao.draw_elements(gl::TRIANGLES);
            // Clear texture
            gl::UseProgram(*clear_texture);
            gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
        }

        for e in gpu_entities {
            println!("size: {:?}",e.size);
        }
        println!();


        glfw.poll_events();
        for (_ ,event) in glfw::flush_messages(&events) {
            use glfw::WindowEvent;
            //use glfw::MouseButton;
            //use glfw::Action;
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

                Key::U => test_entity.gpu_entity.pos = test_entity.gpu_entity.pos - Vec3::Y * state.d_t / 16.,

                _ => (),
            }
        }
        let test_time = time::Instant::now();
        state.window.swap_buffers();

        if test_time.elapsed() > Duration::from_millis(20) {
            use crate::utils::term_colors::*;
            println!("{}buffer swap time: {:?}{}",RED,test_time.elapsed(),RESET_COL);
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
    for handle in generate_thread_handles {
        handle.join().unwrap();
    }
}
fn gen_pos_in_radius(camera_pos: Vec3) -> Vec<IVec3> {
    let camera_pos = camera_pos / chunk::CHUNK_SIZE as f32;
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

fn spawn_generator_thread(
    window:             &glfw::PWindow,
    requests:           Arc<Mutex<mpsc::Receiver<IVec3>>>,
    stop_flag:          Arc<AtomicBool>,
    out_tx:             mpsc::Sender<chunk::Chunk>,
    ) -> std::thread::JoinHandle<()> 
{
    // TODO: when we unload a chunk that didnt have time to load yet it is sill being generated, waste!
    // MAYBE: Check if chunk pos is valid in the thread? Pass target_chunks?
    
    let (mut shared_window, _) = window
        .create_shared(1, 1, "Shared Context 2", glfw::WindowMode::Windowed)
        .expect("Failed to create shared context");
    shared_window.hide();

    thread::spawn(move || {
        shared_window.make_current();
        gl::load_with(|s| shared_window.get_proc_address(s) as *const _);
        while !stop_flag.load(Ordering::Relaxed) {
            let pos = {
                requests
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_millis(10))
            };
            if let Ok(pos) = pos {
                // Now we have `pos` and can perform the remaining work without holding the lock
                let brickmap = chunk::gen_brickmap_2d(pos);
                let (brickmap_grid_ssbo, brickmap_grid_ssbo_addr ,brickmap_data_ssbo,brickmap_data_ssbo_addr) = unsafe { brickmap.gen_ssbos() };

                unsafe { gl::Flush() }; // Finish sending data to ssbo's
                out_tx.send( Entity { 
                    brickmap, 
                    brickmap_data_ssbo, 
                    brickmap_grid_ssbo, 
                    gpu_entity: GPUEntity {
                        brickmap_data_ssbo_addr, 
                        brickmap_grid_ssbo_addr, 
                        pos: (pos * chunk::CHUNK_SIZE as i32).as_vec3(), 
                        _padding1: 7777,
                        orientation: Quaternion::IDENTITY,
                        size: ivec3!(chunk::CHUNK_SIZE),
                        _padding3: 777777,
                    },
                }).unwrap();
            }
        }
    })
}
