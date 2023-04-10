
use std::ptr;
use std::ffi::CString;

use crate::{
    utilities,
    vk::{
        renderpass::*,
        vertex::Vertex
    }

};

pub struct VkGraphicsPipeline {
    pub handle: ash::vk::Pipeline,  
    pub layout: ash::vk::PipelineLayout
}

impl VkGraphicsPipeline {
    pub fn new(
        device: &ash::Device,
        vert_shader_path: Option<&str>,
        frag_shader_path: Option<&str>,
        use_vertex_input: bool,
        render_pass: &VkRenderPass,
        descriptor_set_layouts: &Vec<ash::vk::DescriptorSetLayout>,
        push_constant_ranges: &Vec<ash::vk::PushConstantRange>,
    ) -> Self {
        let mut shader_stages = Vec::new();
        let mut shader_modules = Vec::new();
        
        let main_function_name = CString::new("main").unwrap();

        if let Some(path) = vert_shader_path {
            let vert_shader_code = utilities::read_spirv(std::path::Path::new(path));
            let vert_shader_module = utilities::create_shader_module(device, &vert_shader_code);

            shader_modules.push(vert_shader_module.clone());
            shader_stages.push(ash::vk::PipelineShaderStageCreateInfo {
                s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: ash::vk::PipelineShaderStageCreateFlags::empty(),
                module: vert_shader_module,
                p_name: main_function_name.as_ptr(),
                p_specialization_info: ptr::null(),
                stage: ash::vk::ShaderStageFlags::VERTEX,
            });
        }

        if let Some(path) = frag_shader_path {
            let frag_shader_code = utilities::read_spirv(std::path::Path::new(path));
            let frag_shader_module = utilities::create_shader_module(device, &frag_shader_code);

            shader_modules.push(frag_shader_module.clone());
            shader_stages.push(ash::vk::PipelineShaderStageCreateInfo {
                s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: ash::vk::PipelineShaderStageCreateFlags::empty(),
                module: frag_shader_module,
                p_name: main_function_name.as_ptr(),
                p_specialization_info: ptr::null(),
                stage: ash::vk::ShaderStageFlags::FRAGMENT,
            });
        }

        let binding_description = Vertex::get_binding_descriptions();
        let attribute_description = Vertex::get_attribute_descriptions();

        let vertex_input_state_create_info = if use_vertex_input {
            ash::vk::PipelineVertexInputStateCreateInfo {
                s_type: ash::vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                p_next: ptr::null(),
                flags: ash::vk::PipelineVertexInputStateCreateFlags::empty(),
                vertex_attribute_description_count: attribute_description.len() as u32,
                p_vertex_attribute_descriptions: attribute_description.as_ptr(),
                vertex_binding_description_count: binding_description.len() as u32,
                p_vertex_binding_descriptions: binding_description.as_ptr(),
            }
        } else {
            ash::vk::PipelineVertexInputStateCreateInfo {
                s_type: ash::vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                p_next: ptr::null(),
                flags: ash::vk::PipelineVertexInputStateCreateFlags::empty(),
                vertex_attribute_description_count: 0,
                p_vertex_attribute_descriptions: ptr::null(),
                vertex_binding_description_count: 0,
                p_vertex_binding_descriptions: ptr::null(),
            }
        };

        let vertex_input_assembly_state_info = ash::vk::PipelineInputAssemblyStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            flags: ash::vk::PipelineInputAssemblyStateCreateFlags::empty(),
            p_next: ptr::null(),
            primitive_restart_enable: ash::vk::FALSE,
            topology: ash::vk::PrimitiveTopology::TRIANGLE_LIST,
        };

        let viewports = [ash::vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: ash::vk::Extent2D { width: 1, height: 1 },
        }];

        let viewport_state_create_info = ash::vk::PipelineViewportStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineViewportStateCreateFlags::empty(),
            scissor_count: scissors.len() as u32,
            p_scissors: scissors.as_ptr(),
            viewport_count: viewports.len() as u32,
            p_viewports: viewports.as_ptr(),
        };

        let rasterization_statue_create_info = ash::vk::PipelineRasterizationStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: ash::vk::FALSE,
            cull_mode: ash::vk::CullModeFlags::BACK,
            front_face: ash::vk::FrontFace::CLOCKWISE,
            line_width: 1.0,
            polygon_mode: ash::vk::PolygonMode::FILL,
            rasterizer_discard_enable: ash::vk::FALSE,
            depth_bias_clamp: 0.0,
            depth_bias_constant_factor: 0.0,
            depth_bias_enable: ash::vk::FALSE,
            depth_bias_slope_factor: 0.0,
        };
        let multisample_state_create_info = ash::vk::PipelineMultisampleStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            flags: ash::vk::PipelineMultisampleStateCreateFlags::empty(),
            p_next: ptr::null(),
            rasterization_samples: ash::vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: ash::vk::FALSE,
            min_sample_shading: 0.0,
            p_sample_mask: ptr::null(),
            alpha_to_one_enable: ash::vk::FALSE,
            alpha_to_coverage_enable: ash::vk::FALSE,
        };

        let stencil_state = ash::vk::StencilOpState {
            fail_op: ash::vk::StencilOp::KEEP,
            pass_op: ash::vk::StencilOp::KEEP,
            depth_fail_op: ash::vk::StencilOp::KEEP,
            compare_op: ash::vk::CompareOp::ALWAYS,
            compare_mask: 0,
            write_mask: 0,
            reference: 0,
        };

        let depth_state_create_info = ash::vk::PipelineDepthStencilStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineDepthStencilStateCreateFlags::empty(),
            depth_test_enable: ash::vk::FALSE,
            depth_write_enable: ash::vk::FALSE,
            depth_compare_op: ash::vk::CompareOp::LESS_OR_EQUAL,
            depth_bounds_test_enable: ash::vk::FALSE,
            stencil_test_enable: ash::vk::FALSE,
            front: stencil_state,
            back: stencil_state,
            max_depth_bounds: 1.0,
            min_depth_bounds: 0.0,
        };

        let color_blend_attachment_states = [ash::vk::PipelineColorBlendAttachmentState {
            blend_enable: ash::vk::FALSE,
            color_write_mask: ash::vk::ColorComponentFlags::RGBA,
            src_color_blend_factor: ash::vk::BlendFactor::ONE,
            dst_color_blend_factor: ash::vk::BlendFactor::ZERO,
            color_blend_op: ash::vk::BlendOp::ADD,
            src_alpha_blend_factor: ash::vk::BlendFactor::ONE,
            dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
            alpha_blend_op: ash::vk::BlendOp::ADD,
        }];

        let color_blend_state = ash::vk::PipelineColorBlendStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op_enable: ash::vk::FALSE,
            logic_op: ash::vk::LogicOp::COPY,
            attachment_count: color_blend_attachment_states.len() as u32,
            p_attachments: color_blend_attachment_states.as_ptr(),
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        };

        let dynamic_state = [ash::vk::DynamicState::VIEWPORT, ash::vk::DynamicState::SCISSOR];
        let dynamic_state_info = ash::vk::PipelineDynamicStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_state_count: dynamic_state.len() as u32,
            p_dynamic_states: dynamic_state.as_ptr(),
        };

        let pipeline_layout_create_info = ash::vk::PipelineLayoutCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineLayoutCreateFlags::empty(),
            set_layout_count: descriptor_set_layouts.len() as u32,
            p_set_layouts: descriptor_set_layouts.as_ptr(),
            push_constant_range_count: push_constant_ranges.len() as u32,
            p_push_constant_ranges: push_constant_ranges.as_ptr(),
        };

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_create_info, None).expect("Failed to create pipeline layout!")
        };

        let graphic_pipeline_create_infos = [ash::vk::GraphicsPipelineCreateInfo {
            s_type: ash::vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::PipelineCreateFlags::empty(),
            stage_count: shader_stages.len() as u32,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_state_create_info,
            p_input_assembly_state: &vertex_input_assembly_state_info,
            p_tessellation_state: ptr::null(),
            p_viewport_state: &viewport_state_create_info,
            p_rasterization_state: &rasterization_statue_create_info,
            p_multisample_state: &multisample_state_create_info,
            p_depth_stencil_state: &depth_state_create_info,
            p_color_blend_state: &color_blend_state,
            p_dynamic_state: &dynamic_state_info,
            layout: pipeline_layout,
            render_pass: render_pass.handle,
            subpass: 0,
            base_pipeline_handle: ash::vk::Pipeline::null(),
            base_pipeline_index: -1,
        }];

        let graphics_pipelines = unsafe {
            device.create_graphics_pipelines(ash::vk::PipelineCache::null(), &graphic_pipeline_create_infos, None)
                .expect("Failed to create Graphics Pipeline!.")
        };

        unsafe {
            for shader_module in shader_modules {
                device.destroy_shader_module(shader_module, None);
            }
        }

        Self {  
            handle: graphics_pipelines[0],
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