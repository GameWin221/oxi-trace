use std::ptr;
use std::ffi::CString;

use ash;

use crate::{
    utilities,
    vk::{
        vertex::Vertex,
        buffer::VkBuffer
    }
};

pub struct RayTracingPipeline {
    handle: ash::vk::Pipeline,
    layout: ash::vk::PipelineLayout,
    shader_count: u32,
    rt_pipeline: ash::extensions::khr::RayTracingPipeline
    // BLAS
    // TLAS
    // Scratch Buffer

}

impl RayTracingPipeline {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        chit_shader_path: &str,
        rgen_shader_path: &str,
        miss_shader_path: &str,
        descriptor_set_layouts: &Vec<ash::vk::DescriptorSetLayout>,
        push_constant_ranges: &Vec<ash::vk::PushConstantRange>,
    ) -> RayTracingPipeline{
        let (handle, layout, shader_count, rt_pipeline) = Self::create_pipeline(
            instance,
            device,
            chit_shader_path,
            rgen_shader_path,
            miss_shader_path,
            descriptor_set_layouts,
            push_constant_ranges
        );

        let acceleration_structure = ash::extensions::khr::AccelerationStructure::new(&instance, &device);

        RayTracingPipeline {
            handle,
            layout,
            shader_count,
            rt_pipeline
        }
    }

    fn create_pipeline(
        instance: &ash::Instance,
        device: &ash::Device,
        chit_shader_path: &str,
        rgen_shader_path: &str,
        miss_shader_path: &str,
        descriptor_set_layouts: &Vec<ash::vk::DescriptorSetLayout>,
        push_constant_ranges: &Vec<ash::vk::PushConstantRange>,
    ) -> (ash::vk::Pipeline, ash::vk::PipelineLayout, u32, ash::extensions::khr::RayTracingPipeline) {
        let main_function_name = CString::new("main").unwrap();

        let chit_shader_code = utilities::read_spirv(std::path::Path::new(chit_shader_path));
        let chit_shader_module = utilities::create_shader_module(device, &chit_shader_code);

        let rgen_shader_code = utilities::read_spirv(std::path::Path::new(rgen_shader_path));
        let rgen_shader_module = utilities::create_shader_module(device, &rgen_shader_code);

        let miss_shader_code = utilities::read_spirv(std::path::Path::new(miss_shader_path));
        let miss_shader_module = utilities::create_shader_module(device, &miss_shader_code);

        let shader_modules = vec![
            chit_shader_module.clone(),
            rgen_shader_module.clone(),
            miss_shader_module.clone(),
        ];

        let shader_stages = vec![
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .module(shader_modules[0])
                .name(main_function_name.as_c_str())
                .stage(ash::vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                .build(),
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .module(shader_modules[1])
                .name(main_function_name.as_c_str())
                .stage(ash::vk::ShaderStageFlags::RAYGEN_KHR)
                .build(),
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .module(shader_modules[2])
                .name(main_function_name.as_c_str())
                .stage(ash::vk::ShaderStageFlags::MISS_KHR)
                .build()
        ];

    
        let shader_groups = vec![
            // CHIT
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0)
                .closest_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
            // RGEN
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(1)
                .closest_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
            // MISS
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(ash::vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(3)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
        ];

        let pipeline_layout_create_info = ash::vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(descriptor_set_layouts)
            .push_constant_ranges(push_constant_ranges)
            .build();

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_create_info, None).expect("Failed to create pipeline layout!")
        };

        let rt_pipeline = ash::extensions::khr::RayTracingPipeline::new(&instance, &device);

        let pipeline = unsafe {
            rt_pipeline.create_ray_tracing_pipelines(
                ash::vk::DeferredOperationKHR::null(),
                ash::vk::PipelineCache::null(),
                &[ash::vk::RayTracingPipelineCreateInfoKHR::builder()
                    .stages(&shader_stages)
                    .groups(&shader_groups)
                    .max_pipeline_ray_recursion_depth(1)
                    .layout(pipeline_layout)
                    .build()],
                None,
            )
        }.unwrap()[0];

        unsafe {
            for shader_module in shader_modules {
                device.destroy_shader_module(shader_module, None);
            }
        }

        (pipeline, pipeline_layout, shader_groups.len() as u32, rt_pipeline)
    }

    fn create_shader_binding_table_buffer(
        pipeline: ash::vk::Pipeline,
        rt_pipeline: ash::extensions::khr::RayTracingPipeline,
        shader_count: u32
    ) -> VkBuffer {
        let incoming_table_data = unsafe {
            rt_pipeline.get_ray_tracing_shader_group_handles(
                pipeline,
                0,
                shader_count,
                shader_count as usize * rt_pipeline_properties.shader_group_handle_size as usize,
            )
        }.unwrap();

        let handle_size_aligned = aligned_size(
            rt_pipeline_properties.shader_group_handle_size,
            rt_pipeline_properties.shader_group_base_alignment,
        );

        let table_size = (shader_count  * handle_size_aligned) as usize;
        let mut table_data = vec![0u8; table_size];

        for i in 0..shader_count {
            table_data[i * handle_size_aligned as usize..i * handle_size_aligned as usize + rt_pipeline_properties.shader_group_handle_size as usize].copy_from_slice(
                &incoming_table_data[
                    i * rt_pipeline_properties.shader_group_handle_size as usize..i * 
                    rt_pipeline_properties.shader_group_handle_size as usize + 
                    rt_pipeline_properties.shader_group_handle_size as usize
                ],
            );
        }

        let mut shader_binding_table_buffer = BufferResource::new(
            table_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
            &device,
            device_memory_properties,
        );

        shader_binding_table_buffer.store(&table_data, &device);

        shader_binding_table_buffer
    }

    fn create_blas() {

    }
}