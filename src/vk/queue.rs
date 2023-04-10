

use crate::vk::{sync_objects::*, command_buffer::VkCommandBuffer};

#[derive(Clone, Debug)]
pub struct VkQueue {
    pub handle: ash::vk::Queue,
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
        wait_stages: ash::vk::PipelineStageFlags,
        wait_semaphore: &VkSemaphore,
        signal_semaphore: &VkSemaphore,
        fence: &VkFence
    ) {
        let submit_infos = [ash::vk::SubmitInfo::builder()
            .wait_semaphores(&[wait_semaphore.handle]) 
            .wait_dst_stage_mask(&[wait_stages])
            .command_buffers(&[command_buffer.handle])
            .signal_semaphores(&[signal_semaphore.handle])
            .build()
        ];

        unsafe {
            device.queue_submit(
                self.handle,
                &submit_infos,
                fence.handle,
            ).expect("Failed to execute queue submit.");
        }
    }

    pub fn submit_once(&self,device: &ash::Device, command_buffer: &VkCommandBuffer) {
        let submit_infos = [ash::vk::SubmitInfo {
            s_type: ash::vk::StructureType::SUBMIT_INFO,
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
            device.queue_submit(self.handle, &submit_infos, ash::vk::Fence::null()).expect("Failed to execute single queue submit.");
            device.queue_wait_idle(self.handle).expect("Failed to wait for a queue to go idle!");
        }
    }
}