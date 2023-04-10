
use std::ffi::CString;

use crate::{
    utilities
};

pub struct VkComputePipeline {
    pub handle: ash::vk::Pipeline,  
    pub layout: ash::vk::PipelineLayout
}

impl VkComputePipeline {
    pub fn new(device: &ash::Device,
        compute_shader_path: &str,
        descriptor_set_layouts: &Vec<ash::vk::DescriptorSetLayout>,
        push_constant_ranges: &Vec<ash::vk::PushConstantRange>,
    ) -> Self {
        let shader_entrypoint_name = CString::new("main").unwrap();

        let compute_shader_code = utilities::read_spirv(std::path::Path::new(compute_shader_path));
        let compute_shader_module = utilities::create_shader_module(device, &compute_shader_code);

        let compute_shader_stage = ash::vk::PipelineShaderStageCreateInfo::builder()
            .module(compute_shader_module)
            .name(&shader_entrypoint_name)
            .stage(ash::vk::ShaderStageFlags::COMPUTE)
            .build();
        
        let pipeline_layout_create_info = ash::vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(descriptor_set_layouts)
            .push_constant_ranges(push_constant_ranges)
            .build();

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_create_info, None).expect("Failed to create pipeline layout!")
        };

        let compute_pipeline_create_infos = [ash::vk::ComputePipelineCreateInfo::builder()
            .stage(compute_shader_stage)
            .layout(pipeline_layout)
            .base_pipeline_index(-1) 
            .build()
        ];

        let compute_pipelines = unsafe {
            device.create_compute_pipelines(ash::vk::PipelineCache::null(), &compute_pipeline_create_infos, None)
                .expect("Failed to create Compute Pipeline!.")
        };

        unsafe {
            device.destroy_shader_module(compute_shader_module, None);
        }
        
        Self {  
            handle: compute_pipelines[0],
            layout: pipeline_layout
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.handle, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}