use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use image::DynamicImage;

use crate::{
    utilities,
    vk::{
        buffer::VkBuffer,
        command_buffer::VkCommandBuffer,
        command_pool::VkCommandPool,
        queue::VkQueue
    }
};

pub struct VkTexture {
    pub handle: ash::vk::Image,
    pub view: ash::vk::ImageView,
    pub sampler: Option<ash::vk::Sampler>,
    pub extent: ash::vk::Extent2D,
    pub allocation: Option<Allocation>,

    pub aspect: ash::vk::ImageAspectFlags,
    pub layout: ash::vk::ImageLayout,
}

impl VkTexture {
    pub fn new(
        device: &ash::Device,
        allocator: &mut Allocator,
        img_format: ash::vk::Format,
        extent: ash::vk::Extent2D,
        tiling: ash::vk::ImageTiling,
        usage: ash::vk::ImageUsageFlags,
        aspect: ash::vk::ImageAspectFlags
    ) -> Self {
        let image_info = ash::vk::ImageCreateInfo::builder()
            .image_type(ash::vk::ImageType::TYPE_2D)
            .format(img_format)
            .extent(ash::vk::Extent3D{
                width: extent.width,
                height: extent.height,
                depth: 1
            })
            .tiling(tiling)
            .usage(usage)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .mip_levels(1)
            .array_layers(1)
            .build();

        let image = unsafe { 
            device.create_image(&image_info, None) 
        }.unwrap();

        let requirements = unsafe { 
            device.get_image_memory_requirements(image) 
        };

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name: "Image Allocation",
            requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        }).expect("Failed to allocate an Image!");
        
        unsafe { 
            device.bind_image_memory(image, allocation.memory(), 0).expect("Failed to bind image memory!");
        };

        let image_view_info = ash::vk::ImageViewCreateInfo::builder()
            .view_type(ash::vk::ImageViewType::TYPE_2D)
            .format(img_format)
            .components(ash::vk::ComponentMapping {
                r: ash::vk::ComponentSwizzle::IDENTITY,
                g: ash::vk::ComponentSwizzle::IDENTITY,
                b: ash::vk::ComponentSwizzle::IDENTITY,
                a: ash::vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(ash::vk::ImageSubresourceRange::builder()
                .aspect_mask(aspect)
                .layer_count(1)
                .level_count(1)
                .build()
            )
            .image(image)
            .build();

        let image_view = unsafe { 
            device.create_image_view(&image_view_info, None) 
        }.unwrap();

        Self {
            handle: image,
            view: image_view,
            sampler: None,
            allocation: Some(allocation),
            extent,

            aspect,
            layout: ash::vk::ImageLayout::UNDEFINED
        }
    }

    pub fn create_sampler(&mut self, device: &ash::Device, anisotropy: Option<f32>) {
        let sampler_create_info = ash::vk::SamplerCreateInfo::builder()
            .min_filter(ash::vk::Filter::LINEAR)
            .mag_filter(ash::vk::Filter::LINEAR)
            .mipmap_mode(ash::vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(ash::vk::SamplerAddressMode::REPEAT)
            .address_mode_v(ash::vk::SamplerAddressMode::REPEAT)
            .address_mode_w(ash::vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(anisotropy.is_some())
            .max_anisotropy(anisotropy.unwrap_or(0.0))
            .border_color(ash::vk::BorderColor::INT_OPAQUE_BLACK)
            .build();

        self.sampler = Some(unsafe {
            device.create_sampler(&sampler_create_info, None).expect("Failed to create Sampler!")
        })
    }

    pub fn fill_from_file(&mut self, device: &ash::Device, allocator: &mut Allocator, image_object: &DynamicImage, command_pool: &VkCommandPool, queue: &VkQueue) {  
        let (image_width, image_height) = (image_object.width(), image_object.height());
        let image_size = (image_width * image_height * 4) as ash::vk::DeviceSize;
        let image_data = match &image_object {
            image::DynamicImage::ImageLuma8(_)
            | image::DynamicImage::ImageRgb8(_)
            | image::DynamicImage::ImageLumaA8(_)
            | image::DynamicImage::ImageRgba8(_) => image_object.to_rgba8().into_raw(),
            _ => panic!("Unknown texture format!")
        };

        if image_size <= 0 {
            panic!("Failed to load texture image!")
        }

        let mut staging_buffer = VkBuffer::new(
            device,
            allocator,
            image_size,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu
        );

        staging_buffer.fill(&image_data);

        let cmd = utilities::begin_single_queue_submit(device, command_pool);
        self.transition_layout(device, ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL, &cmd);

        staging_buffer.copy_to_image(&cmd, &self, device);

        self.transition_layout(device, ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, &cmd);
        utilities::end_single_queue_submit(device, command_pool, queue, cmd);

        staging_buffer.destroy(device, allocator);

    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            if let Some(sampler) = self.sampler {
                device.destroy_sampler(sampler, None);
            }

            device.destroy_image_view(self.view, None);
            device.destroy_image(self.handle, None);

            let mut alloc: Option<Allocation> = None;
            std::mem::swap(&mut alloc, &mut self.allocation);
            allocator.free(alloc.unwrap()).expect("Failed to free allocated texture memory!");
        }
    }

    pub fn transition_layout(
        &mut self,
        device: &ash::Device,
        new_layout: ash::vk::ImageLayout,
        cmd: &VkCommandBuffer
    ) {
        cmd.transition_image_layout(device, self.handle, self.aspect, self.layout, new_layout);

        self.layout = new_layout;
    }

    pub fn copy_to_image(&self, device: &ash::Device, command_buffer: &VkCommandBuffer, target_image: ash::vk::Image, target_layout: ash::vk::ImageLayout, target_aspect: ash::vk::ImageAspectFlags) {
        let src_subresource = ash::vk::ImageSubresourceLayers::builder()
            .aspect_mask(self.aspect)
            .layer_count(1)
            .build();
        
        let dst_subresource = ash::vk::ImageSubresourceLayers::builder()
            .aspect_mask(target_aspect)
            .layer_count(1)
            .build();

        
        let region = ash::vk::ImageCopy::builder()
            .src_subresource(src_subresource)
            .dst_subresource(dst_subresource)
            .extent(ash::vk::Extent3D { 
                width: self.extent.width,
                height: self.extent.height,
                depth: 1 
            })
            .build();

        unsafe {
                device.cmd_copy_image(command_buffer.handle, self.handle, self.layout, target_image, target_layout, &[region]);
        }
    }

    pub fn clear(&self, device: &ash::Device, command_buffer: &VkCommandBuffer, clear_color: cgmath::Vector4<f32>) {
        let range = ash::vk::ImageSubresourceRange::builder()
            .aspect_mask(self.aspect)
            .level_count(1)
            .layer_count(1)
            .build();

        let clear_value = ash::vk::ClearColorValue{
            float32: clear_color.into(),
        };

        unsafe {
            device.cmd_clear_color_image(command_buffer.handle, self.handle, self.layout, &clear_value, &[range]);
        }
    }
}