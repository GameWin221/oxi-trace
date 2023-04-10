use crate::vk::{
    surface::*,
    physical_device::*,
    sync_objects::VkSemaphore, queue::VkQueue
};



#[derive(Clone)]
pub struct VkSwapchain {
    pub loader: ash::extensions::khr::Swapchain,
    pub handle: ash::vk::SwapchainKHR,
    pub images: Vec<ash::vk::Image>,
    pub format: ash::vk::Format,
    pub extent: ash::vk::Extent2D,
    pub image_views: Vec<ash::vk::ImageView>,
}

#[derive(Clone, Debug)]
pub struct VkSwapchainSupportDetails {
    pub capabilities: ash::vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<ash::vk::SurfaceFormatKHR>,
    pub present_modes: Vec<ash::vk::PresentModeKHR>,
}

impl VkSwapchain {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: &VkPhysicalDevice,
        surface: &VkSurface,
        desired_extent: ash::vk::Extent2D
    ) -> VkSwapchain {
        let swapchain_support = Self::query_swapchain_support(physical_device, surface);
        let swapchain_format = Self::choose_swapchain_format(&swapchain_support.formats);
        let present_mode = Self::choose_swapchain_present_mode(&swapchain_support.present_modes);
        let extent = Self::choose_swapchain_extent(desired_extent, &swapchain_support.capabilities);

        let image_count = swapchain_support.capabilities.min_image_count + 1;
        let image_count = if swapchain_support.capabilities.max_image_count > 0 {
            image_count.min(swapchain_support.capabilities.max_image_count)
        } else {
            image_count
        };

        let (image_sharing_mode, queue_family_indices) = if physical_device.queue_family_indices.graphics != physical_device.queue_family_indices.present {
            (
                ash::vk::SharingMode::CONCURRENT,
                vec![
                    physical_device.queue_family_indices.graphics.unwrap(),
                    physical_device.queue_family_indices.present.unwrap(),
                ],
            )
        } else {
            (ash::vk::SharingMode::EXCLUSIVE, vec![])
        };

        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_color_space(swapchain_format.color_space)
            .image_format(swapchain_format.format)
            .image_extent(extent)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT | ash::vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(queue_family_indices.as_slice())
            .pre_transform(swapchain_support.capabilities.current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1)
            .build();

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

    pub fn acquire_next_image(&mut self, semaphore: &VkSemaphore) -> Result<(u32, bool), ash::vk::Result> {
        unsafe {
            self.loader.acquire_next_image(self.handle, std::u64::MAX, semaphore.handle, ash::vk::Fence::null())
        }
    }
    pub fn present(&self, image_index: u32, present_queue: &VkQueue, signal_semaphore: &VkSemaphore) -> Result<bool, ash::vk::Result> {
        let present_info = ash::vk::PresentInfoKHR::builder()
            .wait_semaphores(&[signal_semaphore.handle])
            .swapchains(&[self.handle])
            .image_indices(&[image_index])
            .build();

        unsafe {
            self.loader.queue_present(present_queue.handle, &present_info)
        }
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

    fn create_image_views(device: &ash::Device, format: ash::vk::Format,images: &Vec<ash::vk::Image>) -> Vec<ash::vk::ImageView> {
        let swapchain_image_views = images.iter().map(|&image| {
            let create_info = ash::vk::ImageViewCreateInfo::builder()
                .view_type(ash::vk::ImageViewType::TYPE_2D) 
                .format(format)
                .components(ash::vk::ComponentMapping {
                    r: ash::vk::ComponentSwizzle::IDENTITY,
                    g: ash::vk::ComponentSwizzle::IDENTITY,
                    b: ash::vk::ComponentSwizzle::IDENTITY,
                    a: ash::vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(ash::vk::ImageSubresourceRange::builder()
                    .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1)
                    .build()
                )
                .image(image)
                .build();

            unsafe {
                device.create_image_view(&create_info, None).expect("Failed to create Image View!")
            }
        }).collect();

        swapchain_image_views
    }

    fn choose_swapchain_format(available_formats: &Vec<ash::vk::SurfaceFormatKHR>) -> ash::vk::SurfaceFormatKHR {
        if let Some(format) = available_formats.iter().find(
            |&format| format.format == ash::vk::Format::B8G8R8A8_SRGB && format.color_space == ash::vk::ColorSpaceKHR::SRGB_NONLINEAR
        ) {
            format
        } else {
            available_formats.first().unwrap()
        }.clone()
    }

    fn choose_swapchain_present_mode(available_present_modes: &Vec<ash::vk::PresentModeKHR>) -> ash::vk::PresentModeKHR {
        if let Some(&present_mode) = available_present_modes.iter().find(
            |&mode| *mode == ash::vk::PresentModeKHR::MAILBOX
        ) {
            present_mode
        } else {
            ash::vk::PresentModeKHR::FIFO
        }
    }

    fn choose_swapchain_extent(desired_extent: ash::vk::Extent2D, capabilities: &ash::vk::SurfaceCapabilitiesKHR) -> ash::vk::Extent2D {
        if capabilities.current_extent.width != u32::max_value() {
            capabilities.current_extent
        } else {
            ash::vk::Extent2D {
                width: desired_extent.width.clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                height: desired_extent.height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
            }
        }
    }

}