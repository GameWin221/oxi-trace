use ash::vk;
use std::ptr;

use crate::{
    vk_command_pool::VkCommandBuffer,
    vk_framebuffer::VkFramebuffer,
    vk_graphics_pipeline::VkGraphicsPipeline, 
    vk_buffer::VkBuffer
};

#[derive(Clone, Default)]
pub struct VkRenderPass {
    pub handle: vk::RenderPass,

    bound_extent: Option<vk::Extent2D>,
    bound_command_buffer: Option<vk::CommandBuffer>,
    bound_device: Option<ash::Device>,
}

impl VkRenderPass {
    pub fn new(device: &ash::Device, color_attachments: Option<&Vec<(vk::AttachmentDescription, vk::AttachmentReference)>>, depth_attachment: Option<(vk::AttachmentDescription, vk::AttachmentReference)>) -> Self {
        let (mut render_pass_attachments, render_pass_attachment_refs): (Vec<vk::AttachmentDescription>, Vec<vk::AttachmentReference>) = if color_attachments.is_some() {
            (color_attachments.unwrap().iter().map(|(attachment, _attachment_ref)| attachment.clone()).collect(),
            color_attachments.unwrap().iter().map(|(_attachment, attachment_ref)| attachment_ref.clone()).collect())
        } else {
            (vec![], vec![])
        };

        if let Some((attachment, _attachment_ref)) = depth_attachment {
            render_pass_attachments.push(attachment);
        }

        let subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: ptr::null(),
            color_attachment_count: render_pass_attachment_refs.len() as u32,
            p_color_attachments: render_pass_attachment_refs.as_ptr(),
            p_resolve_attachments: ptr::null(),
            p_depth_stencil_attachment: if let Some((_attachment, attachment_ref)) = depth_attachment {
                &attachment_ref
            } else {
                std::ptr::null()
            },
            preserve_attachment_count: 0,
            p_preserve_attachments: ptr::null(),
        };

        let renderpass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            flags: vk::RenderPassCreateFlags::empty(),
            p_next: ptr::null(),
            attachment_count: render_pass_attachments.len() as u32,
            p_attachments: render_pass_attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 0,
            p_dependencies: ptr::null(),
        };

        let render_pass = unsafe {
            device.create_render_pass(&renderpass_create_info, None).expect("Failed to create render pass!")
        };

        VkRenderPass {
            handle: render_pass,
            bound_extent: None,
            bound_command_buffer: None,
            bound_device: None,
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_render_pass(self.handle, None);
        }
    }

    pub fn begin(&mut self, device: &ash::Device, command_buffer: &VkCommandBuffer, framebuffer: &VkFramebuffer) {
        self.bound_command_buffer = Some(command_buffer.handle);
        self.bound_device = Some(device.clone());
        self.bound_extent = Some(framebuffer.extent);

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass: self.handle,
            framebuffer: framebuffer.handle,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: framebuffer.extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        unsafe {
            device.cmd_begin_render_pass(
                self.bound_command_buffer.unwrap(),
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

        }
    }
    pub fn end(&mut self) {
        unsafe {
            self.bound_device.as_ref().unwrap().cmd_end_render_pass(self.bound_command_buffer.unwrap());
        }

        self.bound_extent = None;
        self.bound_device = None;
        self.bound_command_buffer = None;
    }

    pub fn bind_graphics_pipeline(&self, graphics_pipeline: &VkGraphicsPipeline) {
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { 
                width: self.bound_extent.unwrap().width,
                height: self.bound_extent.unwrap().height 
            },
        }];

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.bound_extent.unwrap().width as f32,
            height: self.bound_extent.unwrap().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        
        unsafe {
            self.bound_device.as_ref().unwrap().cmd_bind_pipeline(
                self.bound_command_buffer.unwrap(),
                vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline.handle,
            );
            self.bound_device.as_ref().unwrap().cmd_set_scissor(
                self.bound_command_buffer.unwrap(),
                0,
                &scissors
            );
            self.bound_device.as_ref().unwrap().cmd_set_viewport(
                self.bound_command_buffer.unwrap(),
                0,
                &viewports
            );
        }
    }

    pub fn bind_vertex_buffers(&self, vertex_buffer: &Vec<&VkBuffer>) {
        let buffers: Vec<vk::Buffer> = vertex_buffer.iter().map(|buf| buf.handle).collect();
        let offsets: Vec<vk::DeviceSize> = vertex_buffer.iter().map(|_| 0).collect();

        unsafe {
            self.bound_device.as_ref().unwrap().cmd_bind_vertex_buffers(
                self.bound_command_buffer.unwrap(), 
                0, 
                buffers.as_slice(), 
                offsets.as_slice()
            );
        }
    }
    pub fn bind_index_buffer(&self, index_buffer: &VkBuffer) {
        unsafe {
            self.bound_device.as_ref().unwrap().cmd_bind_index_buffer(
                self.bound_command_buffer.unwrap(), 
                index_buffer.handle, 
                0, 
                vk::IndexType::UINT32
            );
        }
    }

    pub fn draw_indexed(&self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe {
            self.bound_device.as_ref().unwrap().cmd_draw_indexed(
                self.bound_command_buffer.unwrap(),
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance
            );
        }
    }
    pub fn draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.bound_device.as_ref().unwrap().cmd_draw(self.bound_command_buffer.unwrap(), vertex_count, instance_count, first_vertex, first_instance);
        }
    }
}