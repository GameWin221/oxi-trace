use cgmath::{Zero, InnerSpace};
use gpu_allocator::MemoryLocation;

use winit::dpi::PhysicalPosition;
use winit::event::{Event, VirtualKeyCode, ElementState, KeyboardInput, WindowEvent, MouseScrollDelta};
use winit::event_loop::{EventLoop, ControlFlow};

mod vk;
mod utilities;
mod camera;
mod material;

use camera::*;
use material::*;

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
use winit::window::WindowButtons;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
struct SphereRaw {
    position: [f32; 3],
    radius: f32,
    material: [u32; 4]
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct SceneBufferObject {
    materials: [MaterialRaw; 64],
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

    should_reset_rt: bool,
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
            std::mem::size_of::<SceneBufferObject>() as u64,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly
        );

        let camera_buffers: Vec<VkBuffer> = (0..MAX_FRAMES_IN_FLIGHT).into_iter().map(|_|{VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of::<SceneBufferObject>() as u64,
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

        let command_buffers = context.graphics_command_pool.allocate(&context.device, MAX_FRAMES_IN_FLIGHT as u32);

        let image_available_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();
        let render_finished_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();

        let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkFence::new(&context.device, ash::vk::FenceCreateFlags::SIGNALED)).collect();

        let camera = Camera::new(
            cgmath::vec3(2.0, 0.5, 2.0),
            15.0,
            45.0,
            cgmath::vec2(swapchain.extent.width as f32, swapchain.extent.height as f32),
            80.0,
            1.1
        );

        let mut spheres = [SphereRaw::default(); 64];
        spheres[0] = SphereRaw {
            position: [0.0, -100.5, 0.0],
            radius: 100.0,
            material: [0;4]
        };
        spheres[1] = SphereRaw {
            position: [0.0, 0.0, 0.0],
            radius: 0.5,
            material: [1;4]
        };
        spheres[2] = SphereRaw {
            position: [-1.0, 0.0, 0.0],
            radius: 0.5,
            material: [2;4]
        };
        spheres[3] = SphereRaw {
            position: [1.0, 0.0, 0.0],
            radius: 0.5,
            material: [3;4]
        };
        spheres[4] = SphereRaw {
            position: [50.0, 40.0, 50.0],
            radius: 20.0,
            material: [5;4]
        };

        let mut materials = [MaterialRaw::default(); 64];
        materials[0] = Lambertian{
            color: cgmath::vec3(0.7, 0.7, 0.7)
        }.to_raw();
        materials[1] = Lambertian{
            color: cgmath::vec3(0.9, 0.08, 0.1)//cgmath::vec3(0.65, 1.00, 1.0)
        }.to_raw();
        materials[2] = Metal{
            color: cgmath::vec3(0.8, 0.8, 0.8),
            fuzz: 0.3
        }.to_raw();
        materials[3] = Metal{
            color: cgmath::vec3(0.8, 0.6, 0.2),
            fuzz: 1.0
        }.to_raw();
        materials[4] = Dielectric{
            color: cgmath::vec3(1.0, 1.0, 1.0),
            ior: 1.5
        }.to_raw();
        materials[5] = Emmisive{
            color: cgmath::vec3(1.0, 0.4, 0.1),
            intensity: 60.0,
        }.to_raw();

        let scene_buffer_object = SceneBufferObject {
            materials,
            spheres,
            sphere_count: 5
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

        Self {
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

            render_target,
            should_reset_rt: false
        }
    }

    fn update(&mut self, delta_time: f32, mouse_delta: cgmath::Vector2<f32>, scroll_delta: f32, movement_delta: cgmath::Vector3<f32>) {
        self.camera.rotate(mouse_delta.y * delta_time * 10.0, mouse_delta.x * delta_time * 10.0);
        self.camera.translate(movement_delta * delta_time * 8.0);
        self.camera.zoom(scroll_delta * 0.02);

        if mouse_delta.magnitude() > 0.0 || movement_delta.magnitude() > 0.0 || scroll_delta > 0.0{
            self.should_reset_rt = true;
        }
    }

    fn render(&mut self, desired_extent: ash::vk::Extent2D, frame_index: &mut u32) {  
        if desired_extent.width * desired_extent.height == 0 {
            return;
        }

        self.in_flight_fences[self.frame_index].wait(&self.context.device);

        let result = self.swapchain.acquire_next_image(&self.image_available_semaphores[self.frame_index]);

        let (image_index, _is_sub_optimal) = match result {
            Ok(swapchain_info) => swapchain_info,
            Err(result) => match result {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    //self.recreate_swapchain(desired_extent);
                    panic!("Swapchain out of date!");
                    //return;
                }
                _ => panic!("Failed to acquire Swap Chain Image!"),
            },
        };
        
        self.in_flight_fences[self.frame_index].reset(&self.context.device);

        self.camera_buffers[self.frame_index].fill(&[self.camera.to_raw(*frame_index)]);

        self.command_buffers[self.frame_index].begin_recording(&self.context.device, ash::vk::CommandBufferUsageFlags::empty());

        if self.should_reset_rt {
            self.render_target.clear(
                &self.context.device,
                &self.command_buffers[self.frame_index],
                cgmath::vec4(0.2, 0.2, 0.2, 1.0)
            );
            *frame_index = 0;
            self.should_reset_rt = false;
        }
        
        self.command_buffers[self.frame_index].bind_compute_pipeline(&self.context.device, &self.compute_pipeline);

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

        /*let result = */self.swapchain.present(
            image_index,
            &self.context.present_queue,
            &self.render_finished_semaphores[self.frame_index]
        ).expect("Failed to present swapchain image!");

        //let is_resized = match result {
        //    Ok(_) => self.swapchain.extent != desired_extent,
        //    Err(result) => match result {
        //        ash::vk::Result::ERROR_OUT_OF_DATE_KHR | ash::vk::Result::SUBOPTIMAL_KHR => true,
        //        _ => panic!("Failed to execute queue present."),
        //    },
        //};
        //if is_resized {
        //    self.recreate_swapchain(desired_extent);
        //}
        *frame_index += 1;
        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    // Doesnt work and I dont need it for now
    fn _recreate_swapchain(&mut self, desired_extent: ash::vk::Extent2D) {
        if desired_extent.width * desired_extent.height == 0 {
            return;
        }
        
        unsafe {
            self.context.device.device_wait_idle().expect("Failed to wait device idle!");
        };

        self.camera.size = cgmath::vec2(desired_extent.width as f32, desired_extent.height as f32);

        for command_buffer in self.command_buffers.iter() {
            command_buffer.reset(&self.context.device);
        }

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

        println!("Resized to {:?}", self.swapchain.extent);
    }

    pub fn run(mut self, window: winit::window::Window, event_loop: EventLoop<()>) {
        let mut last_frame = std::time::Instant::now();
        let mut frame_index = 0;

        let mut scroll_delta = 0.0;
        let mut mouse_delta = cgmath::vec2(0.0, 0.0);
        let mut movement_delta = cgmath::vec3(0.0, 0.0, 0.0);

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            match input {
                                KeyboardInput { virtual_keycode, state, .. } => {
                                    match virtual_keycode {
                                        Some(VirtualKeyCode::Escape) => {
                                            *control_flow = ControlFlow::Exit
                                        }
                                        Some(VirtualKeyCode::W) => {
                                            movement_delta.z = if state == ElementState::Pressed {
                                                1.0
                                            } else {
                                                0.0
                                            }
                                        }
                                        Some(VirtualKeyCode::S) => {
                                            movement_delta.z = if state == ElementState::Pressed {
                                                -1.0
                                            } else {
                                                0.0
                                            }
                                        }
                                        Some(VirtualKeyCode::A) => {
                                            movement_delta.x = if state == ElementState::Pressed {
                                                -1.0
                                            } else {
                                                0.0
                                            }
                                        }
                                        Some(VirtualKeyCode::D) => {
                                            movement_delta.x = if state == ElementState::Pressed {
                                                1.0
                                            } else {
                                                0.0
                                            }
                                        }
                                        Some(VirtualKeyCode::Space) => {
                                            movement_delta.y = if state == ElementState::Pressed {
                                                1.0
                                            } else {
                                                0.0
                                            }
                                        }
                                        Some(VirtualKeyCode::LControl) => {
                                            movement_delta.y = if state == ElementState::Pressed {
                                                -1.0
                                            } else {
                                                0.0
                                            }
                                        }
                                        _ => {}
                                    }
                                },
                            }
                        }
                        WindowEvent::CursorMoved { position, ..} => {
                            let screen_middle = cgmath::vec2(
                                window.inner_size().width as f32 / 2.0,
                                window.inner_size().height as f32 / 2.0
                            );
                            
                            mouse_delta = cgmath::vec2(
                                position.x as f32 - screen_middle.x,
                                position.y as f32 - screen_middle.y
                            );

                            window.set_cursor_position(PhysicalPosition::new(
                                screen_middle.x,
                                screen_middle.y
                            )).expect("Failed to set cursor grab mode!");
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            if let MouseScrollDelta::LineDelta(x, y) = delta {
                                scroll_delta = y;
                            }
                        }
                        
                        _ => {},
                    }
                },
                Event::MainEventsCleared => {
                    self.update((std::time::Instant::now() - last_frame).as_secs_f32(), mouse_delta, scroll_delta, movement_delta);
                    last_frame = std::time::Instant::now();
                    mouse_delta = cgmath::Vector2::zero();
                    scroll_delta = 0.0;
                    window.request_redraw();
                },
                Event::RedrawRequested(_window_id) => {
                    self.render(ash::vk::Extent2D { 
                        width: window.inner_size().width,
                        height: window.inner_size().height
                    }, &mut frame_index);
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
        .with_resizable(false)
        .with_enabled_buttons(WindowButtons::MINIMIZE | WindowButtons::CLOSE)
        .build(&event_loop)
        .expect("Failed to create window.");

    window.set_cursor_visible(false);

    let monitor = window.primary_monitor().expect("Failed to get the primary monitor!");
    window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(monitor))));

    let app = OxiTrace::new(&window);

    app.run(window, event_loop);
}