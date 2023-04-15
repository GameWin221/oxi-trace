use crate::{vk::{
    context::*,
    compute_pipeline::*,
    command_buffer::*,
    sync_objects::*,
    buffer::*,
    descriptor_pool::*,
    swapchain::*,
    texture::*,
}, 
scene::*, 
utilities, 
camera::*
};

use gpu_allocator::MemoryLocation;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    pub context: VkContext,
    pub swapchain: VkSwapchain,

    compute_pipeline: VkComputePipeline,
    preview_pipeline: VkComputePipeline,
    command_buffers: Vec<VkCommandBuffer>,

    image_available_semaphores: Vec<VkSemaphore>,
    render_finished_semaphores: Vec<VkSemaphore>,
    in_flight_fences: Vec<VkFence>,

    frame_index: usize,
    frames_since_start: u32,

    descriptor_sets: Vec<VkDescriptorSet>,
    scene_buffer: VkBuffer,

    camera_buffers: Vec<VkBuffer>,
    render_target: VkTexture,

    should_reset_rt: bool,
    pub preview_mode: bool,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Renderer {
        let mut context = VkContext::new(window);

        let swapchain = VkSwapchain::new(
            &context.instance,
            &context.device,
            &context.physical_device,
            &context.surface,
            ash::vk::Extent2D { 
                width: window.inner_size().width, 
                height: window.inner_size().height 
            }
        );

        let mut render_target = VkTexture::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(), 
            ash::vk::Format::B8G8R8A8_UNORM,
            ash::vk::Extent2D { 
                width: swapchain.extent.width,
                height: swapchain.extent.height
            },
            ash::vk::ImageTiling::LINEAR,
            ash::vk::ImageUsageFlags::STORAGE | ash::vk::ImageUsageFlags::TRANSFER_SRC | ash::vk::ImageUsageFlags::TRANSFER_DST,
            ash::vk::ImageAspectFlags::COLOR
        );

        let cmd = utilities::begin_single_queue_submit(&context.device, &context.graphics_command_pool);
        for image in swapchain.images.iter() {
            cmd.transition_image_layout(
                &context.device,
                image.clone(),
                ash::vk::ImageAspectFlags::COLOR,
                ash::vk::ImageLayout::UNDEFINED,
                ash::vk::ImageLayout::PRESENT_SRC_KHR
            );
        }
        
        render_target.transition_layout(
            &context.device,
            ash::vk::ImageLayout::GENERAL,
            &cmd
        );
        utilities::end_single_queue_submit(&context.device, &context.graphics_command_pool, &context.graphics_queue, cmd);

        let scene_buffer = VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of::<SceneRaw>() as u64,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly
        );

        let camera_buffers: Vec<VkBuffer> = (0..MAX_FRAMES_IN_FLIGHT).into_iter().map(|_|{VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of::<CameraRaw>() as u64,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu
        )}).collect();

        let descriptor_sets: Vec<VkDescriptorSet> = (0..MAX_FRAMES_IN_FLIGHT).into_iter().map(|i|{
            context.descriptor_pool.allocate(&context.device, &vec![
                VkDescriptorSetSlot{
                    binding: ash::vk::DescriptorSetLayoutBinding {
                        binding: 0,
                        descriptor_type: ash::vk::DescriptorType::STORAGE_IMAGE,
                        descriptor_count: 1,
                        stage_flags: ash::vk::ShaderStageFlags::COMPUTE,
                        p_immutable_samplers: std::ptr::null(),
                    },
                    buffer_info: None,
                    image_info: Some(ash::vk::DescriptorImageInfo{
                        sampler: ash::vk::Sampler::null(),
                        image_view: render_target.view,
                        image_layout: render_target.layout,
                    }),
                },
                VkDescriptorSetSlot{
                    binding: ash::vk::DescriptorSetLayoutBinding {
                        binding: 1,
                        descriptor_type: ash::vk::DescriptorType::UNIFORM_BUFFER,
                        descriptor_count: 1,
                        stage_flags: ash::vk::ShaderStageFlags::COMPUTE,
                        p_immutable_samplers: std::ptr::null(),
                    },
                    buffer_info: Some(ash::vk::DescriptorBufferInfo {
                        buffer: scene_buffer.handle,
                        offset: 0,
                        range: scene_buffer.size,
                    }),
                    image_info: None,
                },
                VkDescriptorSetSlot{
                    binding: ash::vk::DescriptorSetLayoutBinding {
                        binding: 2,
                        descriptor_type: ash::vk::DescriptorType::UNIFORM_BUFFER,
                        descriptor_count: 1,
                        stage_flags: ash::vk::ShaderStageFlags::COMPUTE,
                        p_immutable_samplers: std::ptr::null(),
                    },
                    buffer_info: Some(ash::vk::DescriptorBufferInfo {
                        buffer: camera_buffers[i].handle,
                        offset: 0,
                        range: camera_buffers[i].size,
                    }),
                    image_info: None,
                },
            ])
        }).collect();

        let compute_pipeline = VkComputePipeline::new(
            &context.device,
            "shaders/main.spv",
            &vec![descriptor_sets[0].layout],
            &vec![]
        );
        let preview_pipeline = VkComputePipeline::new(
            &context.device,
            "shaders/preview.spv",
            &vec![descriptor_sets[0].layout],
            &vec![]
        );

        let command_buffers = context.graphics_command_pool.allocate(&context.device, MAX_FRAMES_IN_FLIGHT as u32);

        let image_available_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();
        let render_finished_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();

        let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkFence::new(&context.device, ash::vk::FenceCreateFlags::SIGNALED)).collect();

        Self {
            context,

            swapchain,

            compute_pipeline,
            preview_pipeline,

            command_buffers,

            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,

            frame_index: 0,
            frames_since_start: 0,

            scene_buffer,
            camera_buffers,
            descriptor_sets,

            render_target,
            should_reset_rt: false,
            preview_mode: false,
        }
    }
    pub fn bind_scene(&mut self, scene: &Scene) {
        let mut staging_scene_buffer = VkBuffer::new(
            &self.context.device,
            &mut self.context.allocator.as_mut().unwrap(),
            std::mem::size_of::<SceneRaw>() as u64,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu
        );

        staging_scene_buffer.fill(&[scene.to_raw()]);

        let cmd = utilities::begin_single_queue_submit(&self.context.device, &self.context.transfer_command_pool);
        staging_scene_buffer.copy_to_buffer(&cmd, &self.scene_buffer, &self.context.device);
        utilities::end_single_queue_submit(&self.context.device, &self.context.transfer_command_pool, &self.context.transfer_queue, cmd);

        staging_scene_buffer.destroy(&self.context.device, self.context.allocator.as_mut().unwrap());

    }
    pub fn render(&mut self, camera: &Camera) {  
        if camera.size.x * camera.size.y == 0.0 {
            return;
        }

        self.in_flight_fences[self.frame_index].wait(&self.context.device);

        let result = self.swapchain.acquire_next_image(&self.image_available_semaphores[self.frame_index]);

        let (image_index, _is_sub_optimal) = match result {
            Ok(swapchain_info) => swapchain_info,
            Err(result) => match result {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    panic!("Swapchain out of date!");
                }
                _ => panic!("Failed to acquire Swap Chain Image!"),
            },
        };
        
        self.in_flight_fences[self.frame_index].reset(&self.context.device);

        self.camera_buffers[self.frame_index].fill(&[camera.to_raw(if self.should_reset_rt {
            0
        } else {
            self.frames_since_start
        })]);

        self.command_buffers[self.frame_index].begin_recording(&self.context.device, ash::vk::CommandBufferUsageFlags::empty());

        if self.should_reset_rt {
            self.render_target.clear(
                &self.context.device,
                &self.command_buffers[self.frame_index],
                cgmath::vec4(0.2, 0.2, 0.2, 1.0)
            );
            self.frames_since_start = 0;
            self.should_reset_rt = false;
        }
        
        self.command_buffers[self.frame_index].bind_compute_pipeline(&self.context.device, if self.preview_mode { 
            &self.preview_pipeline
        } else {
            &self.compute_pipeline
        });

        self.command_buffers[self.frame_index].bind_descriptor_set(
            &self.context.device, 
            self.compute_pipeline.layout, 
            &self.descriptor_sets[self.frame_index],
            ash::vk::PipelineBindPoint::COMPUTE
        );

        self.command_buffers[self.frame_index].dispatch(
            &self.context.device,
            (self.swapchain.extent.width as f32 / 4.0).ceil() as u32,
            (self.swapchain.extent.height as f32 / 8.0).ceil() as u32,
            1
        );

        self.command_buffers[self.frame_index].transition_image_layout(
            &self.context.device,
            self.swapchain.images[image_index as usize],
            ash::vk::ImageAspectFlags::COLOR,
            ash::vk::ImageLayout::PRESENT_SRC_KHR,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL
        );

        self.render_target.copy_to_image(
            &self.context.device,
            &self.command_buffers[self.frame_index],
            self.swapchain.images[image_index as usize],
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ash::vk::ImageAspectFlags::COLOR
        );

        self.command_buffers[self.frame_index].transition_image_layout(
            &self.context.device,
            self.swapchain.images[image_index as usize],
            ash::vk::ImageAspectFlags::COLOR,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ash::vk::ImageLayout::PRESENT_SRC_KHR,
        );

        self.command_buffers[self.frame_index].end_recording(&self.context.device);

        self.context.graphics_queue.submit(
            &self.context.device,
            &self.command_buffers[self.frame_index],
            ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            &self.image_available_semaphores[self.frame_index],
            &self.render_finished_semaphores[self.frame_index],
            &self.in_flight_fences[self.frame_index]
        );

        self.swapchain.present(
            image_index,
            &self.context.present_queue,
            &self.render_finished_semaphores[self.frame_index]
        ).expect("Failed to present swapchain image!");

        self.frames_since_start += 1;
        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
    }
    pub fn wait_device_idle(&self) {
        unsafe {
            self.context.device.device_wait_idle().expect("Failed to wait device idle!");
        }
    }
    pub fn reset_render_target(&mut self) {
        self.should_reset_rt = true;
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.render_target.destroy(&self.context.device, self.context.allocator.as_mut().unwrap());

        for descriptor_set in self.descriptor_sets.iter() {
            self.context.descriptor_pool.deallocate(&self.context.device, descriptor_set);
        }
        
        for camera_buffer in self.camera_buffers.iter_mut() {
            camera_buffer.destroy(&self.context.device, self.context.allocator.as_mut().unwrap());
        }

        self.scene_buffer.destroy(&self.context.device, self.context.allocator.as_mut().unwrap());

        for fence in &self.in_flight_fences {
            fence.destroy(&self.context.device);
        }
        for semaphore in &self.render_finished_semaphores {
            semaphore.destroy(&self.context.device);
        }
        for semaphore in &self.image_available_semaphores {
            semaphore.destroy(&self.context.device);
        }

        self.preview_pipeline.destroy(&self.context.device);
        self.compute_pipeline.destroy(&self.context.device);
        self.swapchain.destroy(&self.context.device);
    }
}