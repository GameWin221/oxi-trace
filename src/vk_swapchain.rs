use crate::{
    vk_surface::*,
    vk_physical_device::*,
    vk_sync_objects::VkSemaphore, vk_queue::VkQueue
};

use ash::vk;
use std::ptr;

#[derive(Clone)]
pub struct VkSwapchain {
    pub loader: ash::extensions::khr::Swapchain,
    pub handle: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub image_views: Vec<vk::ImageView>,
}

#[derive(Clone, Debug)]
pub struct VkSwapchainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub fn query_swapchain_support(physical_device: &VkPhysicalDevice, surface: &VkSurface) -> VkSwapchainSupportDetails {
    unsafe {
        let capabilities = surface.loader.get_physical_device_surface_capabilities(physical_device.handle, surface.handle)
            .expect("Failed to query for surface capabilities.");
        let formats = surface.loader.get_physical_device_surface_formats(physical_device.handle, surface.handle)
            .expect("Failed to query for surface formats.");
        let present_modes = surface.loader.get_physical_device_surface_present_modes(physical_device.handle, surface.handle)
            .expect("Failed to query for surface present mode.");

        VkSwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        }
    }
}


impl VkSwapchain {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: &VkPhysicalDevice,
        surface: &VkSurface,
        desired_extent: vk::Extent2D
    ) -> VkSwapchain {
        let swapchain_support = self::query_swapchain_support(physical_device, surface);

        let swapchain_format = Self::choose_swapchain_format(&swapchain_support.formats);
        let present_mode = Self::choose_swapchain_present_mode(&swapchain_support.present_modes);
        let extent = Self::choose_swapchain_extent(desired_extent, &swapchain_support.capabilities);

        let image_count = swapchain_support.capabilities.min_image_count + 1;
        let image_count = if swapchain_support.capabilities.max_image_count > 0 {
            image_count.min(swapchain_support.capabilities.max_image_count)
        } else {
            image_count
        };

        let (image_sharing_mode, queue_family_index_count, queue_family_indices) =
            if physical_device.queue_family_indices.graphics != physical_device.queue_family_indices.present {
                (
                    vk::SharingMode::CONCURRENT,
                    2,
                    vec![
                        physical_device.queue_family_indices.graphics.unwrap(),
                        physical_device.queue_family_indices.present.unwrap(),
                    ],
                )
            } else {
                (vk::SharingMode::EXCLUSIVE, 0, vec![])
            };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface: surface.handle,
            min_image_count: image_count,
            image_color_space: swapchain_format.color_space,
            image_format: swapchain_format.format,
            image_extent: extent,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode,
            p_queue_family_indices: queue_family_indices.as_ptr(),
            queue_family_index_count,
            pre_transform: swapchain_support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            image_array_layers: 1,
        };

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);
        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&swapchain_create_info, None).expect("Failed to create Swapchain!")
        };

        let swapchain_images = unsafe {
            swapchain_loader.get_swapchain_images(swapchain).expect("Failed to get Swapchain Images.")
        };

        let image_views = Self::create_image_views(device, swapchain_format.format, &swapchain_images);

        VkSwapchain {
            loader: swapchain_loader,
            handle: swapchain,
            format: swapchain_format.format,
            extent: extent,
            images: swapchain_images,
            image_views
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            self.image_views.iter().for_each(|&view| {
                device.destroy_image_view(view, None);
            });
    
            self.loader.destroy_swapchain(self.handle, None);
        }
    }

    pub fn acquire_next_image(&mut self, semaphore: &VkSemaphore) -> Result<(u32, bool), vk::Result> {
        unsafe {
            self.loader.acquire_next_image(
                self.handle,
                std::u64::MAX,
                semaphore.handle,
                vk::Fence::null(),
            )
        }
    }

    pub fn present(&self, image_index: u32, present_queue: &VkQueue, signal_semaphore: &VkSemaphore) -> Result<bool, vk::Result> {
        let signal_semaphores = [signal_semaphore.handle];
        let swapchains = [self.handle];

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: ptr::null_mut(),
        };

        unsafe {
            self.loader.queue_present(present_queue.handle, &present_info)
        }
    }

    fn create_image_views(device: &ash::Device, format: vk::Format,images: &Vec<vk::Image>) -> Vec<vk::ImageView> {
        let mut swapchain_imageviews = vec![];

        for &image in images.iter() {
            let imageview_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                view_type: vk::ImageViewType::TYPE_2D,
                format: format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image,
            };

            let imageview = unsafe {
                device.create_image_view(&imageview_create_info, None).expect("Failed to create Image View!")
            };
            swapchain_imageviews.push(imageview);
        }

        swapchain_imageviews
    }

    fn choose_swapchain_format(available_formats: &Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
        if let Some(format) = available_formats.iter().find(
            |&format| format.format == vk::Format::B8G8R8A8_SRGB && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        ) {
            format.clone()
        } else {
            available_formats.first().unwrap().clone()
        }
    }

    fn choose_swapchain_present_mode(available_present_modes: &Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
        if let Some(present_mode) = available_present_modes.iter().find(
            |&mode| *mode == vk::PresentModeKHR::MAILBOX
        ) {
            *present_mode
        } else {
            vk::PresentModeKHR::FIFO
        }
    }

    fn choose_swapchain_extent(desired_extent: vk::Extent2D, capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::max_value() {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: desired_extent.width.clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                height: desired_extent.height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
            }
        }
    }

}