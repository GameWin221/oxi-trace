use gpu_allocator::MemoryLocation;

use winit::event::{Event, VirtualKeyCode, ElementState, KeyboardInput, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};

mod vk;
mod utilities;

use vk::context::*;
use vk::compute_pipeline::*;
use vk::command_buffer::*;
use vk::sync_objects::*;
use vk::buffer::*;
use vk::descriptor_pool::*;
use vk::swapchain::*;
use vk::texture::*;

use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector3};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct UniformBufferObject {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
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
    framebuffer_resized: bool,

    uniform_buffers: Vec<VkBuffer>,
    descriptor_sets: Vec<VkDescriptorSet>,
    model_transform: UniformBufferObject,

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
        utilities::end_single_queue_submit(&context.device, &context.graphics_command_pool, &context.graphics_queue, cmd);

        let mut render_target = VkTexture::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(), 
            ash::vk::Format::B8G8R8A8_UNORM,
            ash::vk::Extent2D { 
                width: window.inner_size().width,
                height: window.inner_size().height
            },
            ash::vk::ImageTiling::LINEAR,
            ash::vk::ImageUsageFlags::STORAGE | ash::vk::ImageUsageFlags::TRANSFER_SRC,
            ash::vk::ImageAspectFlags::COLOR
        );

        let cmd = utilities::begin_single_queue_submit(&context.device, &context.graphics_command_pool);
        render_target.transition_layout(
            &context.device,
            ash::vk::ImageLayout::GENERAL,
            &cmd
        );
        utilities::end_single_queue_submit(&context.device, &context.graphics_command_pool, &context.graphics_queue, cmd);

        let uniform_buffers: Vec<VkBuffer> = (0..swapchain.image_views.len()).into_iter().map(|_| {
            VkBuffer::new(
                &context.device,
                &mut context.allocator.as_mut().unwrap(),
                std::mem::size_of::<UniformBufferObject>() as u64,
                ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                MemoryLocation::CpuToGpu
            )
        }).collect();

        let descriptor_sets: Vec<VkDescriptorSet> = (0..swapchain.image_views.len()).into_iter().map(|_|{
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
                }
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

        let model_transform = UniformBufferObject {
            model: Matrix4::<f32>::identity(),
            view: Matrix4::look_at_rh(
                Point3::new(0.0, 2.0, 2.0),
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
            ),
            proj: cgmath::perspective(
                Deg(45.0),
                swapchain.extent.width as f32
                    / swapchain.extent.height as f32,
                0.1,
                10.0,
            ),
        };

        let mut oxitrace = Self {
            context,

            swapchain,

            compute_pipeline,

            command_buffers,

            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,

            frame_index: 0,
            framebuffer_resized: false,

            uniform_buffers,
            descriptor_sets,
            model_transform,

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

            //self.render_pass.begin(&self.context.device, &command_buffer, &self.framebuffers[i]);

            //self.render_pass.bind_graphics_pipeline(&self.graphics_pipeline);
            //self.render_pass.bind_vertex_buffers(&vec![&self.vertex_buffer]);
            //self.render_pass.bind_index_buffer(&self.index_buffer);
            //self.render_pass.bind_descriptor_set(&self.descriptor_sets[i]);
            //self.render_pass.draw_indexed(INDICES_DATA.len() as u32, 1, 0, 0, 0);

            //self.render_pass.end();

            command_buffer.end_recording(&self.context.device);
        }
    }

    fn update(&mut self) {
        
    }

    fn render(&mut self, desired_extent: ash::vk::Extent2D) {    
        self.in_flight_fences[self.frame_index].wait(&self.context.device);

        let result = self.swapchain.acquire_next_image(&self.image_available_semaphores[self.frame_index]);

        let (image_index, _is_sub_optimal) = match result {
            Ok(image_index) => image_index,
            Err(result) => match result {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    self.recreate_swapchain(desired_extent);
                    return;
                }
                _ => panic!("Failed to acquire Swap Chain Image!"),
            },
        };

        self.update_uniform_buffer(image_index);

        self.in_flight_fences[self.frame_index].reset(&self.context.device);

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
            Ok(_) => self.framebuffer_resized,
            Err(result) => match result {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR | ash::vk::Result::SUBOPTIMAL_KHR => true,
                _ => panic!("Failed to execute queue present."),
            },
        };
        if is_resized {
            self.framebuffer_resized = false;
            self.recreate_swapchain(desired_extent);
        }

        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn update_uniform_buffer(&mut self, current_index: u32) {
        self.uniform_buffers[current_index as usize].fill(&[self.model_transform]);
    }

    fn recreate_swapchain(&mut self, desired_extent: ash::vk::Extent2D) {
        if desired_extent.width * desired_extent.height == 0 {
            return;
        }
        
        unsafe {
            self.context.device.device_wait_idle().expect("Failed to wait device idle!")
        };

        self.swapchain.destroy(&self.context.device);

        self.swapchain = VkSwapchain::new(
            &self.context.instance,
            &self.context.device,
            &self.context.physical_device,
            &self.context.surface,
            desired_extent
        );

        self.render_target.destroy(&self.context.device);
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
                }
            ]);
        }

        Self::record_command_buffer(self);
    }

    pub fn run(mut self, window: winit::window::Window, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            match event {
                | Event::WindowEvent { event, .. } => {
                    match event {
                        | WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit
                        },
                        | WindowEvent::KeyboardInput { input, .. } => {
                            match input {
                                | KeyboardInput { virtual_keycode, state, .. } => {
                                    match (virtual_keycode, state) {
                                        | (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                            *control_flow = ControlFlow::Exit
                                        },
                                        | _ => {},
                                    }
                                },
                            }
                        },
                        | _ => {},
                    }
                },
                | Event::MainEventsCleared => {
                    self.update();
                    window.request_redraw();
                },
                | Event::RedrawRequested(_window_id) => {
                    self.render(ash::vk::Extent2D { 
                        width: window.inner_size().width,
                        height: window.inner_size().height
                    });
                },
                | Event::LoopDestroyed => {
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
        self.render_target.destroy(&self.context.device);

        for descriptor_set in self.descriptor_sets.iter() {
            self.context.descriptor_pool.deallocate(&self.context.device, descriptor_set);
        }
        
        for ub in self.uniform_buffers.iter_mut() {
            ub.destroy(&self.context.device, &mut self.context.allocator.as_mut().unwrap());
        }

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