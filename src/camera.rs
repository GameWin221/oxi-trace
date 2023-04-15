use cgmath::{Vector2, Vector3, InnerSpace};

const WORLD_UP: Vector3<f32> = cgmath::vec3(0.0, 1.0, 0.0);

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct CameraRaw {
    position: [f32; 4],
    horizontal: [f32; 4],
    vertical: [f32; 4],
    lower_left_corner: [f32; 4],
    size: [f32; 2],
    frame_index: [u32; 1],
    exposure: [f32; 1],
}

#[derive(Clone, Debug, Copy)]
pub struct Camera {
    pub position: Vector3<f32>,
    pub pitch: f32,
    pub yaw: f32,
    pub size: Vector2<f32>,
    pub fov: f32,
    pub exposure: f32,
}

impl Camera {
    pub fn new(position: Vector3<f32>, pitch: f32, yaw: f32, size: Vector2<f32>, fov: f32, exposure: f32) -> Self {
        Self { 
            position,
            pitch,
            yaw, 
            size,
            fov,
            exposure
        }
    }

    pub fn rotate(&mut self, pitch: f32, yaw: f32) {
        self.pitch += pitch;
        self.yaw -= yaw;

        self.pitch = self.pitch.clamp(-89.99, 89.99);
    }

    pub fn translate(&mut self, translation: cgmath::Vector3<f32>) {
        let (front, right, up) = self.get_vectors();

        let translation = 
            -translation.x * right +
            translation.y * up +
            -translation.z * front;

        self.position += translation ;
    }

    pub fn zoom(&mut self, delta_fov: f32) {
        self.fov += delta_fov;
        self.fov = self.fov.clamp(10.0, 120.0);
    }

    pub fn to_raw(&self, frames_since_start: u32) -> CameraRaw {
        let aspect_ratio = self.size.x / self.size.y;
        let viewport_height = 2.0 * (self.fov / 2.0).tan();
        let viewport_width = aspect_ratio * viewport_height;

        let (front, right, up) = self.get_vectors();

        let horizontal = viewport_width * right;
        let vertical = viewport_height * up;
        let lower_left_corner = self.position - horizontal/2.0 - vertical/2.0 - front;

        CameraRaw { 
            position: self.position.extend(0.0).into(),
            horizontal: horizontal.extend(0.0).into(),
            vertical: vertical.extend(0.0).into(),
            lower_left_corner: lower_left_corner.extend(0.0).into(),
            size: self.size.into(),
            frame_index: [frames_since_start; 1],
            exposure: [self.exposure; 1]
        }
    }

    fn get_vectors(&self) -> (cgmath::Vector3<f32>, cgmath::Vector3<f32>, cgmath::Vector3<f32>) {
        let front = cgmath::vec3(
			self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
			self.pitch.to_radians().sin(),
			self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
		).normalize();

        let right = WORLD_UP.cross(front).normalize();
        let up = front.cross(right);

        (front, right, up)
    }
}