

#[derive(Clone, Default)]
pub struct VkRenderPass {
    pub handle: ash::vk::RenderPass,
}

impl VkRenderPass {
    pub fn new(device: &ash::Device, color_attachments: Option<&Vec<(ash::vk::AttachmentDescription, ash::vk::AttachmentReference)>>, depth_attachment: Option<(ash::vk::AttachmentDescription, ash::vk::AttachmentReference)>) -> Self {
        let (mut render_pass_attachments, render_pass_attachment_refs): (Vec<ash::vk::AttachmentDescription>, Vec<ash::vk::AttachmentReference>) = if color_attachments.is_some() {
            (color_attachments.unwrap().iter().map(|(attachment, _attachment_ref)| attachment.clone()).collect(),
            color_attachments.unwrap().iter().map(|(_attachment, attachment_ref)| attachment_ref.clone()).collect())
        } else {
            (vec![], vec![])
        };

        if let Some((attachment, _)) = depth_attachment {
            render_pass_attachments.push(attachment);
        }

        let subpass = ash::vk::SubpassDescription {
            flags: ash::vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: ash::vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: render_pass_attachment_refs.len() as u32,
            p_color_attachments: render_pass_attachment_refs.as_ptr(),
            p_depth_stencil_attachment: if let Some((_attachment, attachment_ref)) = depth_attachment {
                &attachment_ref
            } else {
                std::ptr::null()
            },
            ..Default::default()
        };

        let renderpass_create_info = ash::vk::RenderPassCreateInfo::builder()
            .attachments(render_pass_attachments.as_slice())
            .subpasses(&[subpass])
            .build();

        let render_pass = unsafe {
            device.create_render_pass(&renderpass_create_info, None).expect("Failed to create render pass!")
        };

        VkRenderPass {
            handle: render_pass
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_render_pass(self.handle, None);
        }
    }
}