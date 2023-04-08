use ash::vk;

use crate::{
    vk_surface::*
};

#[derive(Clone, Debug)]
pub struct VkQueueFamilyIndices {
    pub graphics: Option<u32>,
    pub present: Option<u32>,
    pub transfer: Option<u32>,
}

impl VkQueueFamilyIndices {
    pub fn new() -> VkQueueFamilyIndices {
        VkQueueFamilyIndices {
            graphics: None,
            present: None,
            transfer: None,
        }
    }

    pub fn find(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface: &VkSurface) -> VkQueueFamilyIndices {
        let queue_families = unsafe { 
            instance.get_physical_device_queue_family_properties(physical_device) 
        };

        let mut queue_family_indices = VkQueueFamilyIndices::new();

        let mut index = 0;
        for queue_family in queue_families.iter() {
            if queue_family.queue_count > 0 && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                queue_family_indices.graphics = Some(index);
            }
            else if queue_family.queue_count > 0 && queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) && !queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                queue_family_indices.transfer = Some(index);
            }

            let is_present_support = unsafe {
                surface.loader.get_physical_device_surface_support(physical_device,index as u32, surface.handle).unwrap()
            };
            if queue_family.queue_count > 0 && is_present_support {
                queue_family_indices.present = Some(index);
            }

            if queue_family_indices.is_complete() {
                break;
            }

            index += 1;
        }

        // Fallback for devices that do not have a separate transfer queue
        if queue_family_indices.transfer.is_none() {
            queue_family_indices.transfer = queue_family_indices.graphics;
        }

        queue_family_indices
    }

    pub fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some() && self.transfer.is_some()
    }
}
