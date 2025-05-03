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
    state.camera.pos = Vec3 { x: 621.59125, y: 320.88193, z: 638.1524 };
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

    //#[allow(unused_mut)]
    //let mut debug_data = vec![0f32;1024];
    //let mut debug_ssbo = 0;
    //let mut ssbo = 0;

    //unsafe {
        //use std::mem;
        //use chunk::BRICK_GRID_SIZE;
        //
        //gl::GenBuffers(1, &mut debug_ssbo);
        //gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, debug_ssbo);
        //gl::BufferData(
            //gl::SHADER_STORAGE_BUFFER,
            //(1024 * mem::size_of::<i32>()) as isize,
            //debug_data.as_ptr() as *const _,
            //gl::DYNAMIC_DRAW,
        //);
        //gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, debug_ssbo);
    //}
    
    state.window.set_size_polling(true);
    state.window.set_key_polling(true);
    state.window.set_cursor_pos_polling(true);
    state.window.set_mouse_button_polling(true);

    let mut time_buffer = utils::TimeBuffer::new(40);

    let (tx, rx) = mpsc::channel();

    let chunk_positions = gen_chunk_pos_in_circle(state.camera.pos/chunk::SIZE as f32,3.5);
    
    thread::spawn(move || {
        use crate::chunk::Chunk;
        shared_window.make_current(); // Make the context current in this thread
        gl::load_with(|s| shared_window.get_proc_address(s) as *const _);
        for pos in chunk_positions {
            let brickmap = chunk::gen_brickmap_2d(pos);
            //let ssbo_time = Instant::now();
            let (brickmap_grid_ssbo, brickmap_data_ssbo) = unsafe { brickmap.gen_ssbos() };
            //println!("ssbo_time {:?}",ssbo_time.elapsed());
            let finish_time = Instant::now();
            unsafe { gl::Flush() };
            println!("flush {:?}",finish_time.elapsed());
            tx.send( Chunk { brickmap, brickmap_data_ssbo, brickmap_grid_ssbo, pos } ).unwrap();
        }
    });

    let mut chunks: Vec<chunk::Chunk> = Vec::new();

    while !state.window.should_close() {
        let frame_time = Instant::now();
        
        let recv_time = Instant::now();
        //match rx.recv_timeout(std::time::Duration::from_micros(1)) {
        match rx.try_recv() {
            Ok(chunk) => {
                chunks.push(chunk);
            },
            _ => (),
        }
        if frame_time.elapsed() > std::time::Duration::from_millis(16) {
            println!("time {:?}",frame_time.elapsed());
        }

        state.input.update(&state.window);
        state.camera.update_with_input(&state.input,state.d_t);
        if state.window.get_cursor_mode() == glfw::CursorMode::Disabled {
            state.window.set_cursor_pos((WIDTH /2 ) as f64, (HEIGHT /2 ) as f64);
        }

        let camera = &state.camera;
        
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
            

            //gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, debug_ssbo);
            //gl::GetBufferSubData(
                //gl::SHADER_STORAGE_BUFFER,
                //0,
                //(debug_data.len() * std::mem::size_of::<i32>()) as _,
                //debug_data.as_mut_ptr() as *mut _,
            //);
            //println!("data len: {}",chunk_brickmap.data.len());
            //println!("og     0: {}",chunk_brickmap.grid[0][0][0]);
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
                        /*
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
                    */
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

        let finish_time = Instant::now();
        //unsafe { gl::Finish() };
        state.window.swap_buffers();

        if finish_time.elapsed() > Duration::from_millis(16) {
            println!("finish time {:?}",finish_time.elapsed());
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
fn gen_chunk_pos_in_circle(camera_pos: Vec3, radius: f32) -> Vec<IVec3> {
    let mut positions = Vec::new();
    let r_squared = radius * radius;

    let min_x = (camera_pos.x - radius).floor() as i32;
    let max_x = (camera_pos.x + radius).ceil() as i32;
    let min_z = (camera_pos.z - radius).floor() as i32;
    let max_z = (camera_pos.z + radius).ceil() as i32;

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
        (camera_pos- (pos * chunk::SIZE as i32 + (chunk::SIZE as i32/2)).as_vec3() ).mag()
    };
    positions.sort_by(|a,b| {
        dist_to_camera(*a).partial_cmp(&dist_to_camera(*b))
        .expect("Coundnt compare")
    });
    positions
}
