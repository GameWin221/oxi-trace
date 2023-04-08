use ash::vk;

use crate::{vk_sync_objects::*, vk_command_pool::VkCommandBuffer};

#[derive(Clone, Debug)]
pub struct VkQueue {
    pub handle: vk::Queue
}

impl VkQueue {
    pub fn new(device: &ash::Device, queue_family_index: u32) -> Self {
        let queue_handle = unsafe { 
            device.get_device_queue(queue_family_index, 0)
        };

        VkQueue {
            handle: queue_handle
        }
    }

    pub fn submit(&self,
        device: &ash::Device,
        command_buffer: &VkCommandBuffer,
        wait_stages: vk::PipelineStageFlags,
        wait_semaphore: &VkSemaphore,
        signal_semaphore: &VkSemaphore,
        fence: &VkFence
    ) {
        let wait_semaphores = [wait_semaphore.handle];
        let wait_stages = [wait_stages];
        let signal_semaphores = [signal_semaphore.handle];

        let submit_infos = [vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &command_buffer.handle,
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
        }];

        unsafe {
            device.queue_submit(
                self.handle,
                &submit_infos,
                fence.handle,
            ).expect("Failed to execute queue submit.");
        }
    }

    pub fn submit_once(&self,device: &ash::Device, command_buffer: &VkCommandBuffer) {
        let submit_infos = [vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &command_buffer.handle,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        }];

        unsafe {
            device.queue_submit(self.handle, &submit_infos, vk::Fence::null()).expect("Failed to execute single queue submit.");
            device.queue_wait_idle(self.handle).expect("Failed to wait for a queue to go idle!");
        }
    }
}