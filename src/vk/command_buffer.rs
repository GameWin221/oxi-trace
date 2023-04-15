use crate::vk::{
    framebuffer::VkFramebuffer,
    buffer::VkBuffer,
    //graphics_pipeline::VkGraphicsPipeline,
    compute_pipeline::VkComputePipeline,
    descriptor_pool::VkDescriptorSet,
    renderpass::VkRenderPass
};



#[derive(Clone, Copy)]
pub struct VkCommandBuffer {
    pub handle: ash::vk::CommandBuffer
}

impl VkCommandBuffer {
    pub fn begin_recording(&self, device: &ash::Device, usage_flags: ash::vk::CommandBufferUsageFlags) {
        let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo {
            s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            p_inheritance_info: std::ptr::null(),
            flags: usage_flags,
        };

        unsafe {
            device.begin_command_buffer(self.handle, &command_buffer_begin_info).expect("Failed to begin recording Command Buffer at beginning!");
        }
    }

    pub fn end_recording(&self, device: &ash::Device) {
        unsafe {
            device.end_command_buffer(self.handle).expect("Failed to record Command Buffer at Ending!");
        }
    }

    pub fn reset(&self, device: &ash::Device) {
        unsafe {
            device.reset_command_buffer(self.handle, ash::vk::CommandBufferResetFlags::RELEASE_RESOURCES).expect("Failed to reset command buffers");
        }
    }

    pub fn begin_renderpass(&mut self, device: &ash::Device, render_pass: &VkRenderPass, framebuffer: &VkFramebuffer) {
        let clear_values = [ash::vk::ClearValue {
            color: ash::vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = ash::vk::RenderPassBeginInfo {
            s_type: ash::vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: std::ptr::null(),
            render_pass: render_pass.handle,
            framebuffer: framebuffer.handle,
            render_area: ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: framebuffer.extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        unsafe {
            device.cmd_begin_render_pass(
                self.handle,
                &render_pass_begin_info,
                ash::vk::SubpassContents::INLINE,
            );

        }
    }
    pub fn end_renderpass(&mut self, device: &ash::Device) {
        unsafe {
            device.cmd_end_render_pass(self.handle);
        }
    }
    /*
    pub fn bind_graphics_pipeline(&mut self, device: &ash::Device, graphics_pipeline: &VkGraphicsPipeline, framebuffer: &VkFramebuffer) {
        let scissors = [ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: ash::vk::Extent2D { 
                width: framebuffer.extent.width,
                height: framebuffer.extent.height 
            },
        }];

        let viewports = [ash::vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: framebuffer.extent.width as f32,
            height: framebuffer.extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        unsafe {
            device.cmd_bind_pipeline(
                self.handle,
                ash::vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline.handle,
            );
            device.cmd_set_scissor(
                self.handle,
                0,
                &scissors
            );
            device.cmd_set_viewport(
                self.handle,
                0,
                &viewports
            );
        }
    }
    */
    pub fn bind_compute_pipeline(&mut self, device: &ash::Device, compute_pipeline: &VkComputePipeline) {
        unsafe {
            device.cmd_bind_pipeline(
                self.handle,
                ash::vk::PipelineBindPoint::COMPUTE,
                compute_pipeline.handle,
            );
        }
    }

    pub fn dispatch(&mut self, device: &ash::Device, x: u32, y: u32, z: u32) {
        unsafe {
            device.cmd_dispatch(self.handle, x, y, z);
        }
    }

    pub fn bind_vertex_buffers(&self, device: &ash::Device, vertex_buffer: &Vec<&VkBuffer>) {
        let buffers: Vec<ash::vk::Buffer> = vertex_buffer.iter().map(|buf| buf.handle).collect();
        let offsets: Vec<ash::vk::DeviceSize> = vertex_buffer.iter().map(|_| 0).collect();

        unsafe {
            device.cmd_bind_vertex_buffers(
                self.handle, 
                0, 
                buffers.as_slice(), 
                offsets.as_slice()
            );
        }
    }
    pub fn bind_index_buffer(&self, device: &ash::Device, index_buffer: &VkBuffer) {
        unsafe {
            device.cmd_bind_index_buffer(
                self.handle, 
                index_buffer.handle, 
                0, 
                ash::vk::IndexType::UINT32
            );
        }
    }

    pub fn bind_descriptor_set(&self, device: &ash::Device, pipeline_layout: ash::vk::PipelineLayout, descriptor_set: &VkDescriptorSet, bind_point: ash::vk::PipelineBindPoint) {
        unsafe {
            device.cmd_bind_descriptor_sets(
                self.handle,
                bind_point,
                pipeline_layout,
                0,
                &[descriptor_set.handle],
                &[]
            );
        }
    }

    pub fn draw_indexed(&self, device: &ash::Device, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe {
            device.cmd_draw_indexed(
                self.handle,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance
            );
        }
    }
    pub fn draw(&self, device: &ash::Device, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            device.cmd_draw(self.handle, vertex_count, instance_count, first_vertex, first_instance);
        }
    }

    pub fn transition_image_layout(
        &self,
        device: &ash::Device,
        image: ash::vk::Image,
        image_aspect: ash::vk::ImageAspectFlags,
        old_layout: ash::vk::ImageLayout,
        new_layout: ash::vk::ImageLayout,
    ) {
        let src_access_mask;
        let dst_access_mask;
        let source_stage;
        let destination_stage;
    
        if old_layout == ash::vk::ImageLayout::UNDEFINED && new_layout == ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            src_access_mask = ash::vk::AccessFlags::empty();
            dst_access_mask = ash::vk::AccessFlags::TRANSFER_WRITE;
            source_stage = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = ash::vk::PipelineStageFlags::TRANSFER;
        } else if old_layout == ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL && new_layout == ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
            src_access_mask = ash::vk::AccessFlags::TRANSFER_WRITE;
            dst_access_mask = ash::vk::AccessFlags::SHADER_READ;
            source_stage = ash::vk::PipelineStageFlags::TRANSFER;
            destination_stage = ash::vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if old_layout == ash::vk::ImageLayout::UNDEFINED && new_layout == ash::vk::ImageLayout::PRESENT_SRC_KHR {
            src_access_mask = ash::vk::AccessFlags::empty();
            dst_access_mask = ash::vk::AccessFlags::COLOR_ATTACHMENT_READ;
            source_stage = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        } else if old_layout == ash::vk::ImageLayout::UNDEFINED && new_layout == ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL {
            src_access_mask = ash::vk::AccessFlags::empty();
            dst_access_mask = ash::vk::AccessFlags::COLOR_ATTACHMENT_READ | ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
            source_stage = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        } else if old_layout == ash::vk::ImageLayout::UNDEFINED && new_layout == ash::vk::ImageLayout::GENERAL {
            src_access_mask = ash::vk::AccessFlags::empty();
            dst_access_mask = ash::vk::AccessFlags::SHADER_READ | ash::vk::AccessFlags::SHADER_WRITE | ash::vk::AccessFlags::TRANSFER_READ | ash::vk::AccessFlags::TRANSFER_WRITE;
            source_stage = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = ash::vk::PipelineStageFlags::ALL_GRAPHICS;
        } else if old_layout == ash::vk::ImageLayout::PRESENT_SRC_KHR && new_layout == ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            src_access_mask = ash::vk::AccessFlags::COLOR_ATTACHMENT_READ;
            dst_access_mask = ash::vk::AccessFlags::TRANSFER_WRITE;
            source_stage = ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            destination_stage = ash::vk::PipelineStageFlags::TRANSFER;
        } else if old_layout == ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL && new_layout == ash::vk::ImageLayout::PRESENT_SRC_KHR {
            src_access_mask = ash::vk::AccessFlags::TRANSFER_WRITE;
            dst_access_mask = ash::vk::AccessFlags::COLOR_ATTACHMENT_READ;
            source_stage = ash::vk::PipelineStageFlags::TRANSFER;
            destination_stage = ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        } else {
            panic!("Unsupported layout transition!")
        }
    
        let image_barriers = [ash::vk::ImageMemoryBarrier::builder()
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .image(image)
            .subresource_range(ash::vk::ImageSubresourceRange::builder()
                .aspect_mask(image_aspect)
                .layer_count(1)
                .level_count(1)
                .build()
            )
            .build()
        ];
    
        unsafe {
            device.cmd_pipeline_barrier(
                self.handle,
                source_stage,
                destination_stage,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_barriers,
            );
        }
    }
}