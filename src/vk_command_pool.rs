use ash::vk;
use std::ptr;

#[derive(Clone, Copy, Debug, Default)]
pub struct VkCommandBuffer {
    pub handle: vk::CommandBuffer,
}

impl VkCommandBuffer {
    pub fn begin_recording(&self, device: &ash::Device, usage_flags: vk::CommandBufferUsageFlags) {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            p_inheritance_info: ptr::null(),
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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VkCommandPool {
    pub handle: vk::CommandPool,
}

impl VkCommandPool {
    pub fn new(device: &ash::Device, queue_family_index: u32) -> Self {
        let command_pool_create_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
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
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: ptr::null(),
            command_buffer_count: count,
            command_pool: self.handle,
            level: vk::CommandBufferLevel::PRIMARY,
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