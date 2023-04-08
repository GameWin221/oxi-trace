use ash::vk;

use crate::vk_renderpass::*;

pub struct VkFramebuffer {
    pub handle: vk::Framebuffer,
    pub extent: vk::Extent2D
}

impl VkFramebuffer {
    pub fn new(device: &ash::Device, extent: vk::Extent2D, views: &Vec<vk::ImageView>, render_pass: &VkRenderPass) -> Self {
        let create_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::FramebufferCreateFlags::empty(),
            render_pass: render_pass.handle,
            attachment_count: views.len() as u32,
            p_attachments: views.as_ptr(),
            width: extent.width,
            height: extent.height,
            layers: 1,
        };

        let framebuffer = unsafe {
            device.create_framebuffer(&create_info, None).expect("Failed to create a framebuffer!")
        };

        Self {
            handle: framebuffer,
            extent
        }
    }
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_framebuffer(self.handle, None);
        }
    }
}