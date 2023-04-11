use cgmath::{Vector2, Vector3, InnerSpace};

const WORLD_UP: Vector3<f32> = cgmath::vec3(0.0, 1.0, 0.0);

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct CameraRaw {
    position: [f32; 4],
    horizontal: [f32; 4],
    vertical: [f32; 4],
    lower_left_corner: [f32; 4],
    size: [f32; 4],
}

#[derive(Clone, Debug, Copy)]
pub struct Camera {
    pub position: Vector3<f32>,
    pub target: Vector3<f32>,
    pub size: Vector2<f32>,
    pub fov: f32
}

impl Camera {
    pub fn new(position: Vector3<f32>, target: Vector3<f32>, size: Vector2<f32>, fov: f32) -> Self {
        Self { 
            position,
            target, 
            size,
            fov 
        }
    }

    pub fn to_raw(&self) -> CameraRaw {
        let aspect_ratio = self.size.x / self.size.y;
        let viewport_height = 2.0 * (self.fov / 2.0).tan();
        let viewport_width = aspect_ratio * viewport_height;

        let w = (self.position - self.target).normalize();
        let u = WORLD_UP.cross(w).normalize();
        let v = w.cross(u);

        let horizontal = viewport_width * u;
        let vertical = viewport_height * v;
        let lower_left_corner = self.position - horizontal/2.0 - vertical/2.0 - w;

        CameraRaw { 
            position: self.position.extend(0.0).into(),
            horizontal: horizontal.extend(0.0).into(),
            vertical: vertical.extend(0.0).into(),
            lower_left_corner: lower_left_corner.extend(0.0).into(),
            size: self.size.extend(0.0).extend(0.0).into(),
        }
    }
}