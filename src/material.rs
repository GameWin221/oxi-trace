#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct MaterialRaw {
    material_type: [u32; 4],
    color: [f32; 3],
    fuzz: [f32; 1],
    emission: [f32; 1],
    ior: [f32; 3],
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Lambertian {
    pub color: cgmath::Vector3<f32>
}

impl Lambertian {
    pub fn to_raw(&self) -> MaterialRaw {
        MaterialRaw {
            material_type: [0;4],
            color: self.color.into(),
            fuzz: [0.0; 1],
            emission: [0.0; 1],
            ior: [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Metal {
    pub color: cgmath::Vector3<f32>,
    pub fuzz: f32
}

impl Metal {
    pub fn to_raw(&self) -> MaterialRaw {
        MaterialRaw {
            material_type: [1;4],
            color: self.color.into(),
            fuzz: [self.fuzz; 1],
            emission: [0.0; 1],
            ior: [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Dielectric {
    pub color: cgmath::Vector3<f32>,
    pub ior: f32
}

impl Dielectric {
    pub fn to_raw(&self) -> MaterialRaw {
        MaterialRaw {
            material_type: [2;4],
            color: self.color.into(),
            fuzz: [0.0; 1],
            emission: [0.0; 1],
            ior: [self.ior; 3],
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Emmisive {
    pub color: cgmath::Vector3<f32>,
    pub intensity: f32
}

impl Emmisive {
    pub fn to_raw(&self) -> MaterialRaw {
        MaterialRaw {
            material_type: [3;4],
            color: self.color.into(),
            fuzz: [0.0; 1],
            emission: [self.intensity; 1],
            ior: [0.0; 3],
        }
    }
}

#[derive(PartialEq)]
pub enum Material {
    Lambertian(Lambertian),
    Dielectric(Dielectric),
    Metal(Metal),
    Emmisive(Emmisive),
}