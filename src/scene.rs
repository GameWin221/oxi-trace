use crate::material::*;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct SphereRaw {
    position: [f32; 3],
    radius: f32,
    material: [u32; 4]
}

pub struct Sphere {
    position: cgmath::Vector3<f32>,
    radius: f32,

    material: u32,
}

impl Sphere {
    pub fn new(position: cgmath::Vector3<f32>, radius: f32, material: u32) -> Sphere {
        Sphere { 
            position,
            radius,
            material 
        }
    }

    pub fn to_raw(&self) -> SphereRaw {
        SphereRaw {
            position: self.position.into(),
            radius: self.radius,
            material: [self.material; 4]
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct SceneRaw {
    materials: [MaterialRaw; 64],
    spheres: [SphereRaw; 64],
    sphere_count: u32
}

pub struct Scene {
    pub materials: Vec<Material>,
    pub spheres: Vec<Sphere>,
}

impl Scene {
    pub fn new(materials: Vec<Material>, spheres: Vec<Sphere>) -> Scene{
        Scene {
            materials,
            spheres
        }
    }
    pub fn to_raw(&self) -> SceneRaw {
        let mut raw_spheres = [SphereRaw::default(); 64];
        let mut raw_materials = [MaterialRaw::default(); 64];
        let mut sphere_count = 0;

        for (i, sphere) in self.spheres.iter().enumerate() {
            raw_spheres[i] = sphere.to_raw();
            sphere_count = i+1;
        }

        for (i, material) in self.materials.iter().enumerate() {
            raw_materials[i] = match material {
                Material::Lambertian(material) => material.to_raw(),
                Material::Metal(material) => material.to_raw(),
                Material::Dielectric(material) => material.to_raw(),
                Material::Emmisive(material) => material.to_raw(),
            };
        }

        SceneRaw {
            materials: raw_materials,
            spheres: raw_spheres,
            sphere_count: sphere_count as u32
        }
    }
}