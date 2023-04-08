use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

#[derive(Clone)]
pub struct VkSurface {
    pub handle: vk::SurfaceKHR,
    pub loader: ash::extensions::khr::Surface,
}

impl VkSurface {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance, window: &winit::window::Window) -> Self {
        let surface = unsafe {
            ash_window::create_surface(entry, instance, window.raw_display_handle(), window.raw_window_handle(), None)
                .expect("Failed to create surface.")   
        };
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);

        VkSurface {
            handle: surface,
            loader: surface_loader
        }
    }

    pub fn destroy(&self) {
        unsafe {
            self.loader.destroy_surface(self.handle, None);
        }
    }
}