use gpu_allocator::MemoryLocation;
use vk_swapchain::VkSwapchain;
use winit::event::{Event, VirtualKeyCode, ElementState, KeyboardInput, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};

use ash::vk;

mod vk_context;
mod vk_swapchain;
mod vk_surface;
mod vk_physical_device;
mod vk_queue_family_indices;
mod vk_debug_messenger;
mod vk_queue;
mod vk_graphics_pipeline;
mod vk_renderpass;
mod vk_framebuffer;
mod vk_command_pool;
mod vk_sync_objects;
mod vk_vertex;
mod vk_buffer;
mod utilities;

use vk_context::*;
use vk_renderpass::*;
use vk_graphics_pipeline::*;
use vk_framebuffer::*;
use vk_command_pool::*;
use vk_sync_objects::*;
use vk_vertex::*;
use vk_buffer::*;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

const VERTICES_DATA: [Vertex; 4] = [
    Vertex {
        pos: [-0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        pos: [0.5, -0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        pos: [0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex {
        pos: [-0.5, 0.5],
        color: [1.0, 1.0, 1.0],
    },
];
const INDICES_DATA: [u32; 6] = [0, 1, 2, 2, 3, 0];

pub struct OxiTrace {
    context: VkContext,

    swapchain: VkSwapchain,

    render_pass: VkRenderPass,
    graphics_pipeline: VkGraphicsPipeline,
    framebuffers: Vec<VkFramebuffer>,

    command_buffers: Vec<VkCommandBuffer>,

    image_available_semaphores: Vec<VkSemaphore>,
    render_finished_semaphores: Vec<VkSemaphore>,
    in_flight_fences: Vec<VkFence>,

    frame_index: usize,
    framebuffer_resized: bool,

    vertex_buffer: VkBuffer,
    index_buffer: VkBuffer,
}

impl OxiTrace {
    pub fn new(window: &winit::window::Window) -> OxiTrace {
        let mut context = VkContext::new(window);

        let swapchain = VkSwapchain::new(
            &context.instance,
            &context.device,
            &context.physical_device,
            &context.surface,
            vk::Extent2D { 
                width: WINDOW_WIDTH, 
                height: WINDOW_HEIGHT 
            }
        );

        let render_pass = VkRenderPass::new(&context.device, Some(&vec![
            (vk::AttachmentDescription {
                flags: vk::AttachmentDescriptionFlags::empty(),
                format: swapchain.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            },
            vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            })
        ]), None);

        let graphics_pipeline = VkGraphicsPipeline::new(
            &context.device,
            Some("shaders/vert.spv"),
            Some("shaders/frag.spv"),
            true,
            &render_pass
        );
        let framebuffers: Vec<VkFramebuffer> = swapchain.image_views.iter().map(
            |&view| VkFramebuffer::new(&context.device, swapchain.extent, &vec![view], &render_pass)
        ).collect();

        let command_buffers = context.graphics_command_pool.allocate(&context.device, framebuffers.len() as u32);

        let image_available_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();
        let render_finished_semaphores = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkSemaphore::new(&context.device)).collect();

        let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT).map(|_| VkFence::new(&context.device, vk::FenceCreateFlags::SIGNALED)).collect();

        let mut staging_vertex_buffer = VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of_val(&VERTICES_DATA) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu
        );

        staging_vertex_buffer.fill(&VERTICES_DATA);

        let vertex_buffer = VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of_val(&VERTICES_DATA) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly
        );

        let cmd = utilities::begin_single_queue_submit(&context.device, &context.transfer_command_pool);
        staging_vertex_buffer.copy_to(&cmd, &vertex_buffer, &context.device);
        utilities::end_single_queue_submit(&context.device, &context.transfer_command_pool, &context.transfer_queue, cmd);

        staging_vertex_buffer.destroy(&context.device, &mut context.allocator.as_mut().unwrap());



        let mut staging_index_buffer = VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of_val(&INDICES_DATA) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu
        );

        staging_index_buffer.fill(&INDICES_DATA);

        let index_buffer = VkBuffer::new(
            &context.device,
            &mut context.allocator.as_mut().unwrap(),
            std::mem::size_of_val(&INDICES_DATA) as u64,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly
        );

        let cmd = utilities::begin_single_queue_submit(&context.device, &context.transfer_command_pool);
        staging_index_buffer.copy_to(&cmd, &index_buffer, &context.device);
        utilities::end_single_queue_submit(&context.device, &context.transfer_command_pool, &context.transfer_queue, cmd);

        staging_index_buffer.destroy(&context.device, &mut context.allocator.as_mut().unwrap());

        let mut oxitrace = Self {
            context,

            swapchain,

            render_pass,
            graphics_pipeline,
            framebuffers,

            command_buffers,

            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,

            frame_index: 0,
            framebuffer_resized: false,

            vertex_buffer,
            index_buffer
        };

        Self::record_command_buffer(&mut oxitrace);

        oxitrace
    }

    fn recreate_swapchain(&mut self, desired_extent: vk::Extent2D) {
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

        for framebuffer in &self.framebuffers {
            framebuffer.destroy(&self.context.device);
        }

        self.framebuffers = self.swapchain.image_views.iter().map(
            |&view| VkFramebuffer::new(&self.context.device, self.swapchain.extent, &vec![view], &self.render_pass)
        ).collect();

        Self::record_command_buffer(self);
    }

    fn record_command_buffer(&mut self) {
        for (i, &command_buffer) in self.command_buffers.iter().enumerate() {
            command_buffer.begin_recording(&self.context.device, vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

            self.render_pass.begin(&self.context.device, &command_buffer, &self.framebuffers[i]);

            self.render_pass.bind_graphics_pipeline(&self.graphics_pipeline);
            self.render_pass.bind_vertex_buffers(&vec![&self.vertex_buffer]);
            self.render_pass.bind_index_buffer(&self.index_buffer);
            self.render_pass.draw_indexed(INDICES_DATA.len() as u32, 1, 0, 0, 0);

            self.render_pass.end();

            command_buffer.end_recording(&self.context.device);
        }
    }

    fn update(&mut self) {
        
    }

    fn render(&mut self, desired_extent: vk::Extent2D) {    
        self.in_flight_fences[self.frame_index].wait(&self.context.device);

        let result = self.swapchain.acquire_next_image(&self.image_available_semaphores[self.frame_index]);

        let (image_index, _is_sub_optimal) = match result {
            Ok(image_index) => image_index,
            Err(vk_result) => match vk_result {
                vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    self.recreate_swapchain(desired_extent);
                    return;
                }
                _ => panic!("Failed to acquire Swap Chain Image!"),
            },
        };

        self.in_flight_fences[self.frame_index].reset(&self.context.device);

        self.context.graphics_queue.submit(
            &self.context.device,
            &self.command_buffers[image_index as usize],
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
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
            Err(vk_result) => match vk_result {
                vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => true,
                _ => panic!("Failed to execute queue present."),
            },
        };
        if is_resized {
            self.framebuffer_resized = false;
            self.recreate_swapchain(desired_extent);
        }

        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
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
                    self.render(vk::Extent2D { 
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
        self.vertex_buffer.destroy(&self.context.device, &mut self.context.allocator.as_mut().unwrap());
        self.index_buffer.destroy(&self.context.device, &mut self.context.allocator.as_mut().unwrap());

        for fence in &self.in_flight_fences {
            fence.destroy(&self.context.device);
        }
        for semaphore in &self.render_finished_semaphores {
            semaphore.destroy(&self.context.device);
        }
        for semaphore in &self.image_available_semaphores {
            semaphore.destroy(&self.context.device);
        }
        
        for framebuffer in &self.framebuffers {
            framebuffer.destroy(&self.context.device);
        }

        self.graphics_pipeline.destroy(&self.context.device);
        self.render_pass.destroy(&self.context.device);
        self.swapchain.destroy(&self.context.device);
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build(&event_loop)
        .expect("Failed to create window.");

    let app = OxiTrace::new(&window);

    app.run(window, event_loop);
}