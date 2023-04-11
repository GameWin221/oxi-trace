use gpu_allocator::MemoryLocation;

use winit::event::{Event, VirtualKeyCode, ElementState, KeyboardInput, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};

mod vk;
mod utilities;
mod camera;

use camera::*;

use vk::{
    context::*,
    compute_pipeline::*,
    command_buffer::*,
    sync_objects::*,
    buffer::*,
    descriptor_pool::*,
    swapchain::*,
    texture::*,
};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
struct SphereRaw {
    position: [f32; 3],
    radius: f32
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct SceneBufferObject {
    spheres: [SphereRaw; 64],
    sphere_count: u32
}

pub struct OxiTrace {
    context: VkContext,
    swapchain: VkSwapchain,

    compute_pipeline: VkComputePipeline,
    command_buffers: Vec<VkCommandBuffer>,

    image_available_semaphores: Vec<VkSemaphore>,
    render_finished_semaphores: Vec<VkSemaphore>,
    in_flight_fences: Vec<VkFence>,

    frame_index: usize,

    camera: Camera,

    descriptor_sets: Vec<VkDescriptorSet>,
    scene_buffer: VkBuffer,

    camera_buffers: Vec<VkBuffer>,

    render_target: VkTexture,
}

impl OxiTrace {
    pub fn new(window: &winit::window::Window) -> OxiTrace {
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
            ash::vk::ImageUsageFlags::STORAGE | ash::vk::ImageUsageFlags::TRANSFER_SRC,
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
            std::mem::size_of::<SceneBufferObject>() as u64,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly
        );

        let camera_buffers: Vec<VkBuffer> = (0..swapchain.image_views.len()).into_iter().map(|_|{VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of::<SceneBufferObject>() as u64,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu
        )}).collect();

        let descriptor_sets: Vec<VkDescriptorSet> = (0..swapchain.image_views.len()).into_iter().map(|i|{
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

        let command_buffers = context.graphics_command_pool.allocate(&context.device, swapchain.image_views.len() as u32);

        let image_available_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();
        let render_finished_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();

        let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkFence::new(&context.device, ash::vk::FenceCreateFlags::SIGNALED)).collect();

        let camera = Camera::new(
            cgmath::vec3(0.0, 0.0, 0.0),
            cgmath::vec3(0.0, 0.0, 0.0),
            cgmath::vec2(swapchain.extent.width as f32, swapchain.extent.height as f32),
            80.0
        );

        let mut spheres = [SphereRaw::default(); 64];
        spheres[0] = SphereRaw {
            position: [0.0, 0.0, 0.0],
            radius: 0.5
        };
        spheres[1] = SphereRaw {
            position: [0.0, -100.5, 0.0],
            radius: 100.0
        };
        spheres[2] = SphereRaw {
            position: [1.5, 0.0, 0.0],
            radius: 0.5
        };

        let scene_buffer_object = SceneBufferObject {
            spheres,
            sphere_count: 3
        };

        let mut staging_scene_buffer = VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of::<SceneBufferObject>() as u64,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu
        );

        staging_scene_buffer.fill(&[scene_buffer_object]);

        let cmd = utilities::begin_single_queue_submit(&context.device, &context.transfer_command_pool);
        staging_scene_buffer.copy_to_buffer(&cmd, &scene_buffer, &context.device);
        utilities::end_single_queue_submit(&context.device, &context.transfer_command_pool, &context.transfer_queue, cmd);

        staging_scene_buffer.destroy(&context.device, context.allocator.as_mut().unwrap());

        let mut oxitrace = Self {
            context,

            swapchain,

            compute_pipeline,

            command_buffers,

            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,

            frame_index: 0,
            camera,

            scene_buffer,
            camera_buffers,
            descriptor_sets,

            render_target
        };

        oxitrace.record_command_buffer();

        oxitrace
    }

    fn record_command_buffer(&mut self) {
        for (i, command_buffer) in self.command_buffers.iter_mut().enumerate() {
            command_buffer.begin_recording(&self.context.device, ash::vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

            command_buffer.bind_compute_pipeline(&self.context.device, &self.compute_pipeline);

            command_buffer.bind_descriptor_set(
                &self.context.device, 
                self.compute_pipeline.layout, 
                &self.descriptor_sets[i],
                ash::vk::PipelineBindPoint::COMPUTE
            );

            command_buffer.dispatch(
                &self.context.device,
                (self.swapchain.extent.width as f32 / 4.0).ceil() as u32,
                (self.swapchain.extent.height as f32 / 8.0).ceil() as u32,
                1
            );

            command_buffer.transition_image_layout(
                &self.context.device,
                self.swapchain.images[i],
                ash::vk::ImageAspectFlags::COLOR,
                ash::vk::ImageLayout::PRESENT_SRC_KHR,
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL
            );

            self.render_target.copy_to_image(
                &self.context.device,
                command_buffer,
                self.swapchain.images[i],
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                ash::vk::ImageAspectFlags::COLOR
            );

            command_buffer.transition_image_layout(
                &self.context.device,
                self.swapchain.images[i],
                ash::vk::ImageAspectFlags::COLOR,
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                ash::vk::ImageLayout::PRESENT_SRC_KHR,
            );

            command_buffer.end_recording(&self.context.device);
        }
    }

    fn update(&mut self, time: f32) {
        let distance = 2.0;
        let x = time.sin() * distance;
        let y = time.sin() * 0.5 + 0.5;
        let z = time.cos() * distance;
        self.camera.position = cgmath::vec3(x, y, z);
    }

    fn render(&mut self, desired_extent: ash::vk::Extent2D) {    
        self.in_flight_fences[self.frame_index].wait(&self.context.device);

        let result = self.swapchain.acquire_next_image(&self.image_available_semaphores[self.frame_index]);

        let (image_index, _is_sub_optimal) = match result {
            Ok(swapchain_info) => swapchain_info,
            Err(result) => match result {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    self.recreate_swapchain(desired_extent);
                    return;
                }
                _ => panic!("Failed to acquire Swap Chain Image!"),
            },
        };
        
        self.in_flight_fences[self.frame_index].reset(&self.context.device);

        self.camera_buffers[image_index as usize].fill(&[self.camera.to_raw()]);

        self.context.graphics_queue.submit(
            &self.context.device,
            &self.command_buffers[image_index as usize],
            ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            &self.image_available_semaphores[self.frame_index],
            &self.render_finished_semaphores[self.frame_index],
            &self.in_flight_fences[self.frame_index]
        );

        let result = self.swapchain.present(
            image_index,
            &self.context.present_queue,
            &self.render_finished_semaphores[self.frame_index]
        );

        let is_resized = match result {
            Ok(_) => self.swapchain.extent != desired_extent,
            Err(result) => match result {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR | ash::vk::Result::SUBOPTIMAL_KHR => true,
                _ => panic!("Failed to execute queue present."),
            },
        };
        if is_resized {
            self.recreate_swapchain(desired_extent);
        }

        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn recreate_swapchain(&mut self, desired_extent: ash::vk::Extent2D) {
        if desired_extent.width * desired_extent.height == 0 {
            return;
        }
        
        unsafe {
            self.context.device.device_wait_idle().expect("Failed to wait device idle!")
        };

        self.camera.size = cgmath::vec2(desired_extent.width as f32, desired_extent.height as f32);

        self.swapchain.destroy(&self.context.device);

        self.swapchain = VkSwapchain::new(
            &self.context.instance,
            &self.context.device,
            &self.context.physical_device,
            &self.context.surface,
            desired_extent
        );
        
        self.render_target.destroy(&self.context.device, self.context.allocator.as_mut().unwrap());
        self.render_target = VkTexture::new(
            &self.context.device,
            &mut self.context.allocator.as_mut().unwrap(), 
            ash::vk::Format::B8G8R8A8_UNORM,
            ash::vk::Extent2D { 
                width: self.swapchain.extent.width,
                height: self.swapchain.extent.height
            },
            ash::vk::ImageTiling::LINEAR,
            ash::vk::ImageUsageFlags::STORAGE | ash::vk::ImageUsageFlags::TRANSFER_SRC,
            ash::vk::ImageAspectFlags::COLOR
        );

        let cmd = utilities::begin_single_queue_submit(&self.context.device, &self.context.graphics_command_pool);
        for image in self.swapchain.images.iter() {
            cmd.transition_image_layout(
                &self.context.device,
                image.clone(),
                ash::vk::ImageAspectFlags::COLOR,
                ash::vk::ImageLayout::UNDEFINED,
                ash::vk::ImageLayout::PRESENT_SRC_KHR
            );
        }
        self.render_target.transition_layout(
            &self.context.device,
            ash::vk::ImageLayout::GENERAL,
            &cmd
        );
        utilities::end_single_queue_submit(&self.context.device, &self.context.graphics_command_pool, &self.context.graphics_queue, cmd);

        for descriptor_set in self.descriptor_sets.iter() {
            descriptor_set.update(&self.context.device, &vec![
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
                        image_view: self.render_target.view,
                        image_layout: self.render_target.layout,
                    }),
                },
            ]);
        }

        println!("Resizing to {:?}", self.swapchain.extent);

        Self::record_command_buffer(self);
    }

    pub fn run(mut self, window: winit::window::Window, event_loop: EventLoop<()>) {
        let start = std::time::Instant::now();
        
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit
                        },
                        WindowEvent::KeyboardInput { input, .. } => {
                            match input {
                                KeyboardInput { virtual_keycode, state, .. } => {
                                    match (virtual_keycode, state) {
                                        (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                            *control_flow = ControlFlow::Exit
                                        },
                                        _ => {},
                                    }
                                },
                            }
                        }
                        _ => {},
                    }
                },
                Event::MainEventsCleared => {
                    self.update((std::time::Instant::now() - start).as_secs_f32());
                    window.request_redraw();
                },
                Event::RedrawRequested(_window_id) => {
                    self.render(ash::vk::Extent2D { 
                        width: window.inner_size().width,
                        height: window.inner_size().height
                    });
                },
                Event::LoopDestroyed => {
                    unsafe {
                        self.context.device.device_wait_idle().expect("Failed to wait device idle!")
                    };
                },
                _ => (),
            }
        })
    }
}

impl Drop for OxiTrace {
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

        self.compute_pipeline.destroy(&self.context.device);
        self.swapchain.destroy(&self.context.device);
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(winit::dpi::LogicalSize::new(1200, 800))
        .build(&event_loop)
        .expect("Failed to create window.");

    let app = OxiTrace::new(&window);

    app.run(window, event_loop);
}