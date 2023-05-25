use crate::material::*;

const MAX_MESHES: usize = 64;
const MAX_VERTICES: usize = 1024*4;
const MAX_INDICES: usize = 1024*16;
const MAX_MATERIALS: usize = 64;
const MAX_SPHERES: usize = 64;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct Vertex {
    position: [f32; 4],
    normal: [f32; 4],
}

impl Vertex {
    pub fn new(position: cgmath::Vector3<f32>, normal: cgmath::Vector3<f32>) -> Vertex {
        Vertex {
            position: position.extend(0.0).into(),
            normal: normal.extend(0.0).into(),
        } 
    }
    pub fn from_raw(position: [f32; 4], normal: [f32; 4]) -> Vertex {
        Vertex {
            position,
            normal,
        } 
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct Mesh {
    vertex_count: u32,
    index_count: u32,
    first_index: u32,
    material_index: u32,
}

impl Mesh {
    pub fn new(vertex_count: u32, index_count: u32, first_index: u32, material_index: u32) -> Mesh {
        Mesh {
            vertex_count,
            index_count,
            first_index,
            material_index
        } 
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct Sphere {
    position: [f32; 3],
    radius: f32,
    material: [u32; 4]
}

impl Sphere {
    pub fn new(position: cgmath::Vector3<f32>, radius: f32, material: u32) -> Sphere {
        Sphere { 
            position: position.into(),
            radius,
            material: [material; 4] 
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct Scene {
    materials: [MaterialRaw; MAX_MATERIALS],
    spheres: [Sphere; MAX_SPHERES],
    sphere_count: [u32; 4],

    vertices: [Vertex; MAX_VERTICES],
    indices: [u32; MAX_INDICES],

    meshes: [Mesh; MAX_MESHES],
    mesh_count: [u32; 4],
}

impl Scene {
    pub fn new(materials: Vec<Material>, spheres: Vec<Sphere>, vertices: Vec<Vertex>, indices: Vec<u32>, meshes: Vec<Mesh>) -> Scene{
        let mut raw_materials = [MaterialRaw::default(); MAX_MATERIALS];

        let mut raw_spheres = [Sphere::default(); MAX_SPHERES];
        let sphere_count = spheres.len();

        let mut raw_vertices = [Vertex::default(); MAX_VERTICES];
        let mut raw_indices = [0; MAX_INDICES];

        let mut raw_meshes = [Mesh::default(); MAX_MESHES];
        let mesh_count = meshes.len();

        for (i, material) in materials.into_iter().enumerate() {
            raw_materials[i] = match material {
                Material::Lambertian(material) => material.to_raw(),
                Material::Metal(material) => material.to_raw(),
                Material::Dielectric(material) => material.to_raw(),
                Material::Emmisive(material) => material.to_raw(),
            };
        }

        for (i, sphere) in spheres.into_iter().enumerate() {
            raw_spheres[i] = sphere;
        }
        for (i, vertex) in vertices.into_iter().enumerate() {
            raw_vertices[i] = vertex;
        }
        for (i, index) in indices.into_iter().enumerate() {
            raw_indices[i] = index;
        }

        for (i, mesh) in meshes.into_iter().enumerate() {
            raw_meshes[i] = mesh;
        }
        
        Scene {
            materials: raw_materials,
            spheres: raw_spheres,
            sphere_count: [sphere_count as u32; 4],
            vertices: raw_vertices,
            indices: raw_indices,
            meshes: raw_meshes,
            mesh_count: [mesh_count as u32; 4]
        }
    }
}