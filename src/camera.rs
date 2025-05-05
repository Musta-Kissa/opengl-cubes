use my_math::prelude::*;
use glfw::Key;
use crate::utils::InputTracker;

#[derive(Clone,Copy)]
pub struct Camera {
    pub pos: Vec3,
    pub up: Vec3,
    pub dir: Vec3,
    pub speed: f32,
    pub near: f32,
    pub far: f32,
    pub fov: f32,
    pub sensitivity: f32,
}
impl Camera {
    pub fn right(&self) -> Vec3 {
        let dir = self.up.cross(self.dir);
        dir.norm()
    }
    pub fn left(&self) -> Vec3 {
        -1. * self.right()
    }
    pub fn forward(&self) -> Vec3 {
        let dir = self.right().cross(self.up);
        dir.norm()
    }
    pub fn back(&self) -> Vec3 {
        -1. * self.forward()
    }
    const MAX_PITCH: f64 = 89.5;
    pub fn update_with_input(&mut self,keys: &InputTracker,d_t:f32) {
        let pitch = self.dir.y.asin().to_degrees() as f64;
        
        let (delta_x,delta_y) = keys.mouse_delta;

        let adjusted_delta_y = delta_y as f32 * self.sensitivity  * (self.fov / 60.);

        let max_delta_y = if (pitch + adjusted_delta_y as f64) >= Self::MAX_PITCH {
            Self::MAX_PITCH - pitch  // Only rotate up to 89
        } else if (pitch + adjusted_delta_y as f64) <= -Self::MAX_PITCH {
            -Self::MAX_PITCH - pitch // Only rotate down to -89
        } else {
            adjusted_delta_y as f64 // Within range, use full delta_y
        };
        if max_delta_y != 0. {
            self.dir.rot_quat(max_delta_y as f32, self.dir.cross(self.up).norm());
        }
        if delta_x != 0. {
            self.dir.rot_quat(delta_x as f32 * self.sensitivity  * (self.fov / 60.), self.up);
        }

        for key in &keys.pressed {
            match key {
                Key::Space => self.pos = self.pos + self.speed  * d_t * self.up, // Up
                Key::LeftShift => self.pos = self.pos - self.speed * d_t * self.up, // Down

                Key::W => self.pos = self.pos + self.speed * d_t * self.forward(),
                Key::S => self.pos = self.pos + self.speed * d_t * self.back(),

                Key::D => self.pos = self.pos + self.speed * d_t * self.right(),
                Key::A => self.pos = self.pos + self.speed * d_t * self.left(),

                Key::K => {
                    self.speed = self.speed * (1. + 0.05 * d_t / 16.);
                    println!("speed: {}", self.speed);
                }
                Key::J => {
                    self.speed = self.speed * (1. - 0.05 * d_t / 16.);
                    println!("speed: {}", self.speed);
                }

                Key::O => self.fov *= 1. + 0.05 * d_t/16.,
                Key::P => self.fov *= 1. - 0.05 * d_t/16.,

                Key::C => {
                    println!("self.pos:{:?}", self.pos);
                    println!("self.dir:{:?}", self.dir);
                }
                Key::R => {
                    self.up = vec3!(0.,1.,0.); 
                    self.fov = 60.;
                }
                Key::E => self.up.rot_quat(-1. * d_t / 16., self.dir),    //Roll Right
                Key::Q => self.up.rot_quat(1. * d_t / 16., self.dir),     //Roll Left
                Key::Left => self.dir.rot_quat(-1. * (d_t / 16.) * (self.fov / 60.), self.up), //Yaw Left
                Key::Right => self.dir.rot_quat(1. * (d_t / 16.) * (self.fov / 60.), self.up), //Yaw Right
                Key::Up => {
                    // clamp the angle
                    if self.dir.dot(self.up) >= 1.0 - 1e-4 {
                        continue;
                    }
                    //Pitch Up
                    self.dir.rot_quat(1. * (d_t / 16.) * (self.fov / 120.), self.dir.cross(self.up).norm());
                }
                Key::Down => {
                    // clamp the angle
                    if self.dir.dot(self.up) <= -1.0 + 1e-4{
                        continue;
                    }
                    //Pitch Down
                    self.dir.rot_quat(-1. * (d_t / 16.) * (self.fov / 120.), self.dir.cross(self.up).norm());
                }

                _ => (),
            }
        }

    }
}
impl Default for Camera {
    fn default() -> Camera {
        Camera {
            up: vec3!(0., 1., 0.),
            //pos: vec3!(1e-5,1e-5,-5. + 1e-5),
            //dir: vec3!(1e-7,1e-7,1.).norm(),
            pos:Vec3 { x: 13.870341, y: 12.413917, z: 4.1491528 },
            dir:Vec3 { x: -0.5975243, y: -0.6896208, z: 0.40913445 },
            speed: 0.135,
            near: 0.1,
            far: 100.,
            fov: 60.,
            sensitivity: 0.1,
        }
    }
}
