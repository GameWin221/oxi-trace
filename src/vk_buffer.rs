use ash::vk;

use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use crate::vk_command_pool::VkCommandBuffer;

#[derive(Debug)]
pub struct VkBuffer {
    pub handle: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub size: vk::DeviceSize,
}

impl VkBuffer {
    pub fn new(device: &ash::Device, allocator: &mut Allocator, size: vk::DeviceSize, usage: vk::BufferUsageFlags, mem_location: MemoryLocation) -> Self {
        let vk_info = vk::BufferCreateInfo::builder().size(size).usage(usage).build();

        let buffer = unsafe { 
            device.create_buffer(&vk_info, None) 
        }.unwrap();
        let requirements = unsafe { 
            device.get_buffer_memory_requirements(buffer) 
        };

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name: "Memory Buffer Allocation",
            requirements,
            location: mem_location,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        }).expect("Failed to allocate a Buffer!");
        
        unsafe { 
            device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()).expect("Failed to bind buffer memory!") 
        };

        Self {
            handle: buffer,
            allocation: Some(allocation),
            size
        }
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        let mut alloc: Option<Allocation> = None;
        std::mem::swap(&mut alloc, &mut self.allocation);
        
        allocator.free(alloc.unwrap()).unwrap();

        unsafe { 
            device.destroy_buffer(self.handle, None) 
        };
    }

    pub fn fill<T>(&mut self, data: &[T]) {
        unsafe {
            let dst_ptr = self.allocation.as_ref().unwrap().mapped_ptr().unwrap().cast().as_ptr();

            std::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, data.len());
        }
    }
    pub fn copy_to(&mut self, command_buffer: &VkCommandBuffer, other: &Self, device: &ash::Device) {
        unsafe {
            device.cmd_copy_buffer(command_buffer.handle, self.handle, other.handle, &[vk::BufferCopy{
                src_offset: 0,
                dst_offset: 0,
                size: self.size,
            }]);
        }
    }
}