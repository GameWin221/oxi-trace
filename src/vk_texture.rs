use ash::vk;

use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use image::DynamicImage;

use crate::utilities;
use crate::vk_buffer::VkBuffer;
use crate::vk_command_pool::VkCommandBuffer;
use crate::vk_command_pool::VkCommandPool;
use crate::vk_queue::VkQueue;

pub struct VkTexture {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub sampler: Option<vk::Sampler>,
    pub extent: vk::Extent2D,
    pub allocation: Allocation,

    pub aspect: vk::ImageAspectFlags,
    pub layout: vk::ImageLayout,
}

impl VkTexture {
    pub fn new(
        device: &ash::Device,
        allocator: &mut Allocator,
        img_format: vk::Format,
        extent: vk::Extent2D,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        aspect: vk::ImageAspectFlags
    ) -> Self {
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(img_format)
            .extent(vk::Extent3D{
                width: extent.width,
                height: extent.height,
                depth: 1
            })
            .tiling(tiling)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
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

        let image_view_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(img_format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange::builder()
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
            allocation,
            extent,

            aspect,
            layout: vk::ImageLayout::UNDEFINED
        }
    }

    pub fn create_sampler(&mut self, device: &ash::Device, anisotropy: Option<f32>) {
        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .min_filter(vk::Filter::LINEAR)
            .mag_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(anisotropy.is_some())
            .max_anisotropy(anisotropy.unwrap_or(0.0))
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .build();

        self.sampler = Some(unsafe {
            device.create_sampler(&sampler_create_info, None).expect("Failed to create Sampler!")
        })
    }

    pub fn fill_from_file(&mut self, device: &ash::Device, allocator: &mut Allocator, image_object: &DynamicImage, command_pool: &VkCommandPool, queue: &VkQueue) {  
        let (image_width, image_height) = (image_object.width(), image_object.height());
        let image_size = (image_width * image_height * 4) as vk::DeviceSize;
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
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu
        );

        staging_buffer.fill(&image_data);

        let cmd = utilities::begin_single_queue_submit(device, command_pool);
        self.transition_layout(device, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &cmd);

        staging_buffer.copy_to_image(&cmd, &self, device);

        self.transition_layout(device, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, &cmd);
        utilities::end_single_queue_submit(device, command_pool, queue, cmd);

        staging_buffer.destroy(device, allocator);

    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            if let Some(sampler) = self.sampler {
                device.destroy_sampler(sampler, None);
            }

            device.destroy_image_view(self.view, None);
            device.destroy_image(self.handle, None);
        }
    }

    pub fn transition_layout(
        &mut self,
        device: &ash::Device,
        new_layout: vk::ImageLayout,
        cmd: &VkCommandBuffer
    ) {
        let src_access_mask;
        let dst_access_mask;
        let source_stage;
        let destination_stage;

        if self.layout == vk::ImageLayout::UNDEFINED && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            src_access_mask = vk::AccessFlags::empty();
            dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        } else if self.layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
            src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            dst_access_mask = vk::AccessFlags::SHADER_READ;
            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if self.layout == vk::ImageLayout::UNDEFINED && new_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL {
            src_access_mask = vk::AccessFlags::empty();
            dst_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        } else {
            panic!("Unsupported layout transition!")
        }

        let image_barriers = [vk::ImageMemoryBarrier::builder()
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .old_layout(self.layout)
            .new_layout(new_layout)
            .image(self.handle)
            .subresource_range(vk::ImageSubresourceRange::builder()
                .aspect_mask(self.aspect)
                .layer_count(1)
                .level_count(1)
                .build()
            )
            .build()
        ];

        unsafe {
            device.cmd_pipeline_barrier(
                cmd.handle,
                source_stage,
                destination_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_barriers,
            );
        }

        self.layout = new_layout;
    }
}