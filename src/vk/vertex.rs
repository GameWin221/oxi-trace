

use memoffset::offset_of;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn get_binding_descriptions() -> [ash::vk::VertexInputBindingDescription; 1] {
        [ash::vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: ash::vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn get_attribute_descriptions() -> [ash::vk::VertexInputAttributeDescription; 3] {
        [
            ash::vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: ash::vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            ash::vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            },
            ash::vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: ash::vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, tex_coord) as u32,
            },
        ]
    }
}