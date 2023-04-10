

use crate::vk::renderpass::*;

pub struct VkFramebuffer {
    pub handle: ash::vk::Framebuffer,
    pub extent: ash::vk::Extent2D
}

impl VkFramebuffer {
    pub fn new(device: &ash::Device, extent: ash::vk::Extent2D, views: &Vec<ash::vk::ImageView>, render_pass: &VkRenderPass) -> Self {
        let create_info = ash::vk::FramebufferCreateInfo {
            s_type: ash::vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::FramebufferCreateFlags::empty(),
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