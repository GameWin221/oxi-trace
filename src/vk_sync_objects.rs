use ash::vk;

pub struct VkSemaphore {
    pub handle: vk::Semaphore
}

impl VkSemaphore {
    pub fn new(device: &ash::Device) -> Self {
        let create_info = vk::SemaphoreCreateInfo::builder().build();

        Self {
            handle: unsafe { device.create_semaphore(&create_info, None).expect("Failed to create Semaphore Object!") }
        }
    }
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_semaphore(self.handle, None);
        }
    }
}

pub struct VkFence {
    pub handle: vk::Fence
}

impl VkFence {
    pub fn new(device: &ash::Device, create_flags: vk::FenceCreateFlags) -> Self {
        let create_info = vk::FenceCreateInfo::builder().flags(create_flags).build();

        Self {
            handle: unsafe { 
                device.create_fence(&create_info, None).expect("Failed to create Fence Object!") 
            }
        }
    }
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_fence(self.handle, None);
        }
    }

    pub fn wait(&self, device: &ash::Device) {
        unsafe {
            device.wait_for_fences(&[self.handle], true, std::u64::MAX).expect("Failed to wait for Fence!");
        }
    }

    pub fn reset(&self, device: &ash::Device) {
        unsafe {
            device.reset_fences(&[self.handle]).expect("Failed to reset Fence!");
        }
    }
}