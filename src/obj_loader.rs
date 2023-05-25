use crate::{scene::Vertex};
use cgmath::vec3;
use tobj::{self};

pub fn load_from_file(path: &str) -> Result<(Vec<Vertex>, Vec<u32>), ()> {
    let cornell_box = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS);

    let (models, materials) = cornell_box.expect("Failed to load OBJ file");

    let materials = materials.unwrap_or_default();

    let mesh = &models.first().expect("Model contains no meshes!").mesh;

    if mesh.normals.is_empty() || mesh.texcoords.is_empty() {
        panic!("Normals and texture coordinates are required!");
    }

    let positions: Vec<[f32; 4]> = mesh.positions.chunks(3).map(|i| [i[0], i[1], i[2], 0.0]).collect();
    let normals: Vec<[f32; 4]> = mesh.normals.chunks(3).map(|i| [i[0], i[1], i[2], 0.0]).collect();

    let mut vertices = Vec::with_capacity(mesh.positions.len());

    assert!(positions.len() == normals.len());

    for i in 0..positions.len() {
        vertices.push(Vertex::from_raw(positions[i], normals[i]));
    }

    println!("Vertex count: {}, Index count: {}", vertices.len(), mesh.indices.len());

    Ok((vertices, mesh.indices.clone()))
}
