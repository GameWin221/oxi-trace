use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use crate::vk::command_buffer::VkCommandBuffer;
use crate::vk::texture::VkTexture;

#[derive(Debug)]
pub struct VkBuffer {
    pub handle: ash::vk::Buffer,
    pub allocation: Option<Allocation>,
    pub size: ash::vk::DeviceSize,
}

impl VkBuffer {
    pub fn new(device: &ash::Device, allocator: &mut Allocator, size: ash::vk::DeviceSize, usage: ash::vk::BufferUsageFlags, mem_location: MemoryLocation) -> Self {
        let info = ash::vk::BufferCreateInfo::builder().size(size).usage(usage).build();

        let buffer = unsafe { 
            device.create_buffer(&info, None) 
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
    
    pub fn copy_to_buffer(&mut self, command_buffer: &VkCommandBuffer, other: &Self, device: &ash::Device) {
        unsafe {
            device.cmd_copy_buffer(command_buffer.handle, self.handle, other.handle, &[ash::vk::BufferCopy{
                src_offset: 0,
                dst_offset: 0,
                size: self.size,
            }]);
        }
    }
    pub fn copy_to_image(&mut self, command_buffer: &VkCommandBuffer, other: &VkTexture, device: &ash::Device) {
        let buffer_image_regions = [ash::vk::BufferImageCopy::builder()
        .image_subresource(ash::vk::ImageSubresourceLayers {
            aspect_mask: ash::vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        })
        .image_extent(ash::vk::Extent3D {
            width: other.extent.width,
            height: other.extent.height,
            depth: 1,
        }).build()];
    
        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer.handle,
                self.handle,
                other.handle,
                other.layout,
                &buffer_image_regions,
            );
        }
    }
}