use cgmath::{Zero, InnerSpace};

use renderer::Renderer;
use scene::{Scene, Sphere};
use winit::dpi::{PhysicalPosition};
use winit::event::{Event, VirtualKeyCode, ElementState, KeyboardInput, WindowEvent, MouseScrollDelta};
use winit::event_loop::{EventLoop, ControlFlow};

mod vk;
mod utilities;
mod camera;
mod material;
mod renderer;
mod scene;

use camera::*;
use material::*;

use winit::window::WindowButtons;

pub struct OxiTrace {
    camera: Camera,
    scene: Scene,
    renderer: Renderer,

    scroll_delta: f32,
    mouse_delta: cgmath::Vector2<f32>,
    movement_delta: cgmath::Vector3<f32>,
}

impl OxiTrace {
    pub fn new(window: &winit::window::Window) -> OxiTrace {
        let mut renderer = Renderer::new(window);

        let camera = Camera::new(
            cgmath::vec3(2.0, 0.5, 2.0),
            15.0,
            45.0,
            cgmath::vec2(
                renderer.swapchain.extent.width as f32,
                renderer.swapchain.extent.height as f32
            ),
            80.0,
            1.1
        );

        let spheres = vec![
            Sphere::new(cgmath::vec3(0.0, -100.5, 0.0), 100.0, 0),
            Sphere::new(cgmath::vec3(0.0, 0.0, 0.0), 0.5, 1),
            Sphere::new(cgmath::vec3(-1.0, 0.0, 0.0), 0.5, 2),
            Sphere::new(cgmath::vec3(1.0, 0.0, 0.0), 0.5, 3),
            Sphere::new(cgmath::vec3(50.0, 40.0, 50.0), 20.0, 5),
        ];

        let materials = vec![
            Material::Lambertian(Lambertian {
                color: cgmath::vec3(0.7, 0.7, 0.7) 
            }),
            Material::Lambertian(Lambertian {
                color: cgmath::vec3(0.9, 0.08, 0.1)
            }),
            Material::Metal(Metal { 
                color: cgmath::vec3(0.8, 0.8, 0.8),
                fuzz: 0.3
            }),
            Material::Metal(Metal {
                color: cgmath::vec3(0.8, 0.6, 0.2),
                fuzz: 1.0 
            }),
            Material::Dielectric(Dielectric {
                color: cgmath::vec3(1.0, 1.0, 1.0),
                ior: 1.5
            }),
            Material::Emmisive(Emmisive{
                color: cgmath::vec3(1.0, 0.4, 0.1),
                intensity: 60.0,
            })
        ];

        let scene = Scene::new(materials, spheres);

        renderer.bind_scene(&scene);

        Self {
            camera,
            scene,
            renderer,

            scroll_delta: 0.0,
            mouse_delta: cgmath::vec2(0.0, 0.0),
            movement_delta: cgmath::vec3(0.0, 0.0, 0.0),
        }
    }

    fn update(&mut self, delta_time: f32) {
        self.camera.rotate(self.mouse_delta.y * delta_time * 10.0, self.mouse_delta.x * delta_time * 10.0);
        self.camera.translate(self.movement_delta * delta_time * 8.0);
        self.camera.zoom(self.scroll_delta * 0.02);

        if self.mouse_delta.magnitude() > 0.0 || self.movement_delta.magnitude() > 0.0 || self.scroll_delta > 0.0{
            self.renderer.reset_render_target();
        }

        self.mouse_delta = cgmath::Vector2::zero();
        self.scroll_delta = 0.0;
    }
    fn render(&mut self) {  
        self.renderer.render(&self.camera);
    }
   
    fn wait_device_idle(&self) {
        self.renderer.wait_device_idle();
    }

    fn process_keyboard_input(&mut self, key: VirtualKeyCode, state: ElementState, control_flow: &mut ControlFlow) {
        match key {
            VirtualKeyCode::Escape => {
                *control_flow = ControlFlow::Exit
            }
            VirtualKeyCode::R => {
                if state == ElementState::Released {
                    self.renderer.preview_mode = !self.renderer.preview_mode;
                    self.renderer.reset_render_target();
                }
            }
            VirtualKeyCode::W => {
                self.movement_delta.z = if state == ElementState::Pressed {
                    1.0
                } else {
                    0.0
                }
            }
            VirtualKeyCode::S => {
                self.movement_delta.z = if state == ElementState::Pressed {
                    -1.0
                } else {
                    0.0
                }
            }
            VirtualKeyCode::A => {
                self.movement_delta.x = if state == ElementState::Pressed {
                    -1.0
                } else {
                    0.0
                }
            }
            VirtualKeyCode::D => {
                self.movement_delta.x = if state == ElementState::Pressed {
                    1.0
                } else {
                    0.0
                }
            }
            VirtualKeyCode::Space => {
                self.movement_delta.y = if state == ElementState::Pressed {
                    1.0
                } else {
                    0.0
                }
            }
            VirtualKeyCode::LControl => {
                self.movement_delta.y = if state == ElementState::Pressed {
                    -1.0
                } else {
                    0.0
                }
            }
            _ => {}
        }
    }
    fn process_scroll_input(&mut self, x: f32, y: f32) {
        self.mouse_delta = cgmath::vec2(x, y);
    }
    fn process_mouse_movement(&mut self, position: PhysicalPosition<f64>, screen_middle: PhysicalPosition<f32>) {
        self.mouse_delta = cgmath::vec2(
            position.x as f32 - screen_middle.x,
            position.y as f32 - screen_middle.y
        );
    }

    pub fn run(mut self, window: winit::window::Window, event_loop: EventLoop<()>) {
        let mut last_frame = std::time::Instant::now();

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
                                    if let Some(key) = virtual_keycode {
                                        self.process_keyboard_input(key, state, control_flow);
                                    }
                                },
                            }
                        }
                        WindowEvent::CursorMoved { position, ..} => {
                            let screen_middle = PhysicalPosition::new(
                                window.inner_size().width as f32 / 2.0,
                                window.inner_size().height as f32 / 2.0,
                            );

                            self.process_mouse_movement(position, screen_middle);      

                            window.set_cursor_position(screen_middle).expect("Failed to set cursor grab mode!");
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            if let MouseScrollDelta::LineDelta(x, y) = delta {
                                self.process_scroll_input(x, y);
                            }
                        }
                        
                        _ => {},
                    }
                },
                Event::MainEventsCleared => {
                    self.update((std::time::Instant::now() - last_frame).as_secs_f32());
                    last_frame = std::time::Instant::now();
                    window.request_redraw();
                },
                Event::RedrawRequested(_window_id) => {
                    self.render();
                },
                Event::LoopDestroyed => {
                    self.wait_device_idle();
                },
                _ => (),
            }
        })
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_title("OxiTrace")
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