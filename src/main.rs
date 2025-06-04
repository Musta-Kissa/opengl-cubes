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
mod entity;
mod allocator;

#[macro_use]
extern crate my_math;
use my_math::prelude::*;

use glfw::{Context, Key, PWindow};
use std::{time,thread::{self,JoinHandle}};
use std::time::{Instant,Duration};
use std::sync::mpsc;
use crate::utils::*;
use crate::entity::Entity;
use std::sync::{Arc,Mutex,RwLock, atomic::{AtomicBool, Ordering}};

use camera::Camera;

pub const HEIGHT: u32 = 1000;
pub const WIDTH: u32 = HEIGHT * 16/9;

pub const FPS: f64 = 60.;//f64::MAX;
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

use crate::allocator::BrickAllocator;
use std::sync::OnceLock;

fn main() {
    let (mut glfw, win, events) = unsafe { utils::init(WIDTH,HEIGHT) };


    unsafe { allocator::BrickAllocator::init(2*1024_u32.pow(3)/std::mem::size_of::<chunk::Brick>() as u32) };

    let mut state = AppState::with_window(win);
    //state.camera.pos= vec3!(1900./3.5+ 256.0,
                            //256 as f32 *  1.5,
                            //1900./3.5+ 512.0);
    //state.camera.pos = Vec3 { x: chunk::SIZE as f32 / 2., y: 220.88193, z: chunk::SIZE as f32 / 2.};
    state.camera.pos = vec3!(15.,313.,12.);
    state.camera.dir = vec3!(1.,-0.5004213,0.003213).norm();
    state.camera.speed = 64.;


    // Load shaders
    let (screen_texturing_program,dda_program,clear_texture,draw_entity_program) = unsafe {
        use crate::shader::*;
        let uv_passthrough_vert         = compile_shader(gl::VERTEX_SHADER,"./shaders/screen.vert");
        let texturig_frag               = compile_shader(gl::FRAGMENT_SHADER,"./shaders/texturing.frag");
        let dda_compute_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/dda_brick.comp");
        let clear_texture_shader        = compile_shader(gl::COMPUTE_SHADER,"./shaders/clear_texture.comp");
        let draw_entity_shader          = compile_shader(gl::COMPUTE_SHADER,"./shaders/draw_entities.comp");

        let screen_texturing_program    = ShaderProgram::create_program(uv_passthrough_vert,texturig_frag);
        let dda_program                 = ShaderProgram::create_compute(dda_compute_shader);
        let clear_texture               = ShaderProgram::create_compute(clear_texture_shader);
        let draw_entity_program         = ShaderProgram::create_compute(draw_entity_shader);

        gl::DeleteShader(uv_passthrough_vert);
        gl::DeleteShader(texturig_frag);
        gl::DeleteShader(dda_compute_shader);
        gl::DeleteShader(clear_texture_shader);
        (screen_texturing_program,dda_program,clear_texture,draw_entity_program)
    };

    let texture = create_texture(WIDTH,HEIGHT);
    unsafe { gl::BindImageTexture(0, texture, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F) };
    let screen_vao = unsafe {
        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao);
        vao
    };
    state.window.set_size_polling(true);
    state.window.set_key_polling(true);
    state.window.set_cursor_pos_polling(true);
    state.window.set_mouse_button_polling(true);

    let mut time_buffer = utils::TimeBuffer::new(40);

    let (out_tx, out_rx) = mpsc::channel();
    let (request_tx, request_rx) = mpsc::channel();
    let request_rx = Arc::new(Mutex::new(request_rx));

    let target_chunk_pos = Arc::new(RwLock::new(gen_pos_in_radius(state.camera.pos)));
    for pos in target_chunk_pos.read().unwrap().iter() {
        request_tx.send(*pos).unwrap();
    }

    let generate_thread_stop_flag = Arc::new(AtomicBool::new(false));
    let generate_thread_handles:Vec<JoinHandle<()>> = 
        (0..GENERATOR_THREAD_COUNT).map(|_| 
            spawn_generator_thread(
                &state.window,
                Arc::clone(&request_rx),
                Arc::clone(&generate_thread_stop_flag),
                Arc::clone(&target_chunk_pos),
                out_tx.clone(),
        )).collect();


    let mut entities = Entities::new();
    let mut chunks: Vec<u32> = Vec::new();
    let mut entity_handle = entities.add(entity::gen_entity());

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
                let handle = entities.add(chunk);
                chunks.push(handle);
            },
            _ => (),
        }

        // UPDATE CHUNKS
        {
            let mut change_flag = false;
            let camera_pos = camera.pos / chunk::SIZE as f32;
            let r_squared = CHUNK_RADIUS*CHUNK_RADIUS;

            // REMOVE CHUNKS
            let mut i = 0;
            while i < chunks.len() {
                let pos = entities.get(chunks[i]).pos / chunk::SIZE as f32;
                let dx = pos.x as f32 + 0.5 - camera_pos.x;
                let dz = pos.z as f32 + 0.5 - camera_pos.z;
                if (dx*dx + dz*dz) <= r_squared { // CHUNK POS IS STILL VALID
                    i+=1;
                } else { // REMOVE CHUNK
                    unsafe {
                        let chunk = entities.get(chunks[i]);
                        let time = std::time::Instant::now();
                        //chunk.free_bricks();
                        println!("free chunk bricks {:?}",time.elapsed());
                        gl::DeleteBuffers(1, &chunk.brickmap_grid_ssbo);
                        //gl::DeleteBuffers(1, &chunk.brickmap_data_ssbo);
                    }
                    entities.remove(chunks[i]);
                    chunks.swap_remove(i);
                    change_flag = true;
                }
            }
            target_chunk_pos.write().unwrap().retain(|pos| {
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
                    if (dx*dx + dz*dz) <= r_squared && !target_chunk_pos.read().unwrap().contains(&ivec3!(x,0,z)) {
                        pos_to_add.push(ivec3!(x,0,z));
                        change_flag = true;
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

            let mut target_chunk_pos_guard = target_chunk_pos.write().unwrap();
            for pos in pos_to_add {
                target_chunk_pos_guard.push(pos);
                let _ = request_tx.send(pos);
            }
            drop(target_chunk_pos_guard);

            if change_flag {
                println!("CHUNK NUMBER: {} TARGER: {}",chunks.len(),target_chunk_pos.read().unwrap().len());
            }
        }
        
        let dist_to_camera = |pos: Vec3| {
            (camera.pos - ( pos * chunk::SIZE as f32 + (chunk::SIZE as f32/2.) ) ).mag()
        };
        //chunks.sort_by(|a,b| {
            //dist_to_camera(entities.get(*a).pos / chunk::SIZE as f32).partial_cmp(&dist_to_camera(entities.get(*b).pos / chunk::SIZE as f32))
            //.expect("Coundnt compare")
        //});
        chunks.sort_by(|a,b| {
            dist_to_camera(entities.get(*a).pos / chunk::SIZE as f32).partial_cmp(&dist_to_camera(entities.get(*b).pos / chunk::SIZE as f32))
            .expect("Coundnt compare")
        });

        // RENDER /////////////////////////////////////////////////////////////////////////////////////////////////////////

        unsafe {
            gl::UseProgram(*draw_entity_program);

            draw_entity_program.set_vec3("light_dir",state.light_dir);
            draw_entity_program.set_float("fov",camera.fov);
            for entity_handle in entities.depth_sorted_handles(camera.pos) {
                let entity = entities.get(entity_handle);
                let (local_ray_pos,local_ray_dir) = entity::ray_to_local(entity,camera.pos,camera.dir);

                draw_entity_program.set_ivec3("ENTITY_SIZE",entity.size);
                draw_entity_program.set_vec3("camera_pos",local_ray_pos);
                draw_entity_program.set_vec3("camera_dir",local_ray_dir);

                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, entity.brickmap_grid_ssbo);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, allocator::BRICK_ALLOCATOR().data_ssbo);

                gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
            }

            /*
            gl::UseProgram(*dda_program);
            dda_program.set_float("fov",camera.fov);
            dda_program.set_int("CHUNK_SIZE",chunk::SIZE as i32);
            dda_program.set_vec3("camera_pos",camera.pos);
            dda_program.set_vec3("camera_dir",camera.dir);
            dda_program.set_vec3("light_dir",state.light_dir);
            
            // Color texture
            for chunk in &chunks {
                let chunk = entities.get(*chunk);
                dda_program.set_vec3("CHUNK_POS",chunk.pos);

                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, chunk.brickmap_grid_ssbo);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, chunk.brickmap_data_ssbo);

                gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
            }
            */
            
            // Draw texture
            gl::UseProgram(*screen_texturing_program);
            gl::BindVertexArray(screen_vao);
            gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);

            // Clear texture
            gl::UseProgram(*clear_texture);
            gl::DispatchCompute(WIDTH /16 +1, HEIGHT/16 +1, 1);
            //gl::MemoryBarrier(gl::ALL_BARRIER_BITS);
        }


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

                Key::U => {
                    let entity = entities.get(entity_handle);
                    entity.pos = entity.pos - Vec3::X * state.d_t / 16.;
                }

                Key::C => println!("light_dir {:?}",state.light_dir),
                _ => (),
            }
        }
        let test_time = time::Instant::now();
        state.window.swap_buffers();

        if test_time.elapsed() > Duration::from_millis(20) {
            use crate::utils::colors::*;
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

fn spawn_generator_thread(
    window:             &glfw::PWindow,
    requests:           Arc<Mutex<mpsc::Receiver<IVec3>>>,
    stop_flag:          Arc<AtomicBool>,
    target_chunk_pos:   Arc<RwLock<Vec<IVec3>>>,
    out_tx:             mpsc::Sender<Entity>,
    ) -> std::thread::JoinHandle<()> 
{
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
                // check if we still want to generate the chunk
                if !target_chunk_pos.read().unwrap().contains(&pos) {
                    continue;
                }
                // Now we have `pos` and can perform the remaining work without holding the lock
                let brickmap_grid = chunk::gen_chunk_brickmap(pos);
                //let (brickmap_grid_ssbo, brickmap_data_ssbo) = unsafe { entity::brickmap_gen_ssbos(&brickmap_grid,&brickmap_data) };
                let brickmap_grid_ssbo = unsafe { entity::brickmap_gen_ssbos(&brickmap_grid) };

                unsafe { gl::Flush() }; // Finish sending data to ssbo's
                out_tx.send( Entity { 
                    brickmap_grid, 
                    brickmap_grid_ssbo, 
                    pos: (pos * chunk::SIZE as i32).into(),
                    orientation: Quaternion::new(1.0,Vec3::ZERO), 
                    size: ivec3!(chunk::SIZE) 
                }).unwrap();
            }
        }
    })
}

struct Entities {
    inner: Vec<Option<Entity>>,
    free_list: Vec<u32>,
}
impl Entities {
    pub fn new() -> Self {
        Entities { inner: Vec::new(), free_list: Vec::new() }
    }
    pub fn add(&mut self,entity: Entity) -> u32 {
        if let Some(idx) = self.free_list.pop() {
            self.inner[idx as usize] = Some(entity);
            return idx;
        } else {
            let idx = self.inner.len();
            self.inner.push(Some(entity));
            return idx as u32;
        }
    }
    pub fn remove(&mut self, idx: u32) {
        if idx == self.inner.len() as u32 -1 {
            self.inner.pop();
        } else {
            self.inner[idx as usize] = None;
            self.free_list.push(idx);
        }
    }
    pub fn get(&mut self,idx: u32) -> &mut Entity {
        self.inner[idx as usize].as_mut().unwrap()
    }
    pub fn depth_sorted_handles(&mut self, camera_pos: Vec3) -> Vec<u32> {
        let mut all_handles: Vec<u32> = self.inner
            .iter()
            .enumerate()
            .filter_map(|(i, val)| if val.is_some() { Some(i as u32) } else { None })
            .collect();

        let dist_to_camera = |pos: Vec3| {
            (camera_pos - ( pos * chunk::SIZE as f32 + (chunk::SIZE as f32/2.) ) ).mag()
        };
        all_handles.sort_by(|a,b| {
            dist_to_camera(self.get(*a).pos / chunk::SIZE as f32).partial_cmp(&dist_to_camera(self.get(*b).pos / chunk::SIZE as f32))
            .expect("Coundnt compare")
        });
        all_handles
    }
}
