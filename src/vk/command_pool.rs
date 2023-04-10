
use std::ptr;

use crate::vk::command_buffer::VkCommandBuffer;

#[derive(Clone, Copy, Debug, Default)]
pub struct VkCommandPool {
    pub handle: ash::vk::CommandPool,
}

impl VkCommandPool {
    pub fn new(device: &ash::Device, queue_family_index: u32) -> Self {
        let command_pool_create_info = ash::vk::CommandPoolCreateInfo {
            s_type: ash::vk::StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index,
        };

        let command_pool = unsafe {
            device.create_command_pool(&command_pool_create_info, None).expect("Failed to create Command Pool!")
        };

        VkCommandPool { 
            handle: command_pool
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_command_pool(self.handle, None);
        }
    }

    pub fn allocate(&self, device: &ash::Device, count: u32) -> Vec<VkCommandBuffer> {
        let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo {
            s_type: ash::vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: ptr::null(),
            command_buffer_count: count,
            command_pool: self.handle,
            level: ash::vk::CommandBufferLevel::PRIMARY,
        };

        unsafe {
            device.allocate_command_buffers(&command_buffer_allocate_info).expect("Failed to allocate Command Buffers!").iter().map(
                |&native| VkCommandBuffer {
                    handle: native
                }
            ).collect()
        }
    }

    pub fn deallocate(&self, device: &ash::Device, target: &VkCommandBuffer) {
        unsafe {
            device.free_command_buffers(self.handle, &[target.handle]);
        }
    }
}