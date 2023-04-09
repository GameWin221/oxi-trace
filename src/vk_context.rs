use raw_window_handle::HasRawDisplayHandle;

use crate::{
    vk_debug_messenger::*,
    vk_queue_family_indices::*,
    vk_physical_device::*, 
    vk_surface::*,
    vk_queue::*,
    vk_command_pool::*,
    vk_descriptor_pool::*,
};

use gpu_allocator::vulkan::*;

use ash::vk;
use std::collections::HashSet;
use std::ffi::CString;
use std::os::raw::{c_void, c_char};
use std::ptr;

pub const APPLICATION_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);
pub const ENGINE_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);
pub const API_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);

pub const WINDOW_TITLE: &'static str = "OxiTrace";

pub struct VkContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    
    pub allocator: Option<Allocator>,

    pub surface: VkSurface,
    pub debug_messenger: VkDebugMessenger,
    pub physical_device: VkPhysicalDevice,

    pub graphics_queue: VkQueue,
    pub present_queue: VkQueue,
    pub transfer_queue: VkQueue,

    pub graphics_command_pool: VkCommandPool,
    pub transfer_command_pool: VkCommandPool,

    pub descriptor_pool: VkDescriptorPool
}

impl VkContext {
    pub fn new(window: &winit::window::Window) -> VkContext {
        let entry = ash::Entry::linked();
        let instance = Self::create_instance(&entry, window.raw_display_handle());
        let debug_messenger = VkDebugMessenger::new(&entry, &instance);

        let surface = VkSurface::new(&entry, &instance, &window);
        let physical_device = VkPhysicalDevice::new(&instance, &surface);

        let (device, queue_families) = Self::create_logical_device(&instance, &physical_device, &surface);
        let graphics_queue = VkQueue::new(&device, queue_families.graphics.unwrap());
        let present_queue = VkQueue::new(&device, queue_families.present.unwrap());
        let transfer_queue = VkQueue::new(&device, queue_families.transfer.unwrap());

        let graphics_command_pool = VkCommandPool::new(&device, queue_families.graphics.unwrap());
        let transfer_command_pool = VkCommandPool::new(&device, queue_families.transfer.unwrap());

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: physical_device.handle,
            debug_settings: Default::default(),
            buffer_device_address: true,
        }).expect("Failed to create a Vulkan Memory Allocator");

        let descriptor_pool = VkDescriptorPool::new(&device);

        VkContext { 
            entry, 
            instance,
            device,

            allocator: Some(allocator),

            surface,
            debug_messenger,
            physical_device,

            graphics_queue,
            present_queue,
            transfer_queue,

            graphics_command_pool,
            transfer_command_pool,

            descriptor_pool
        }
    }

    fn create_instance(entry: &ash::Entry, raw_display_handle: raw_window_handle::RawDisplayHandle) -> ash::Instance {
        if ENABLE_VALIDATION_LAYERS && check_validation_layer_support(entry) == false {
            panic!("Validation layers requested, but not available!");
        }

        let app_name = CString::new(WINDOW_TITLE).unwrap();
        let engine_name = CString::new("Vulkan Engine").unwrap();
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            application_version: APPLICATION_VERSION,
            p_engine_name: engine_name.as_ptr(),
            engine_version: ENGINE_VERSION,
            api_version: API_VERSION, // set api_version to vk_make_version!(2, 0, 92) to test if the p_next field in vk::InstanceCreateInfo works.
        };

        // This create info used to debug issues in vk::createInstance and vk::destroyInstance.
        let debug_utils_create_info = populate_debug_messenger_create_info();

        // VK_EXT debug utils has been requested here.
        let mut extension_names = ash_window::enumerate_required_extensions(raw_display_handle)
            .expect("Failed to enumerate required instance extensions.")
            .to_vec();
        
        if ENABLE_VALIDATION_LAYERS {
            extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        }

        let required_validation_layer_raw_names: Vec<CString> = VALIDATION_LAYERS.iter().map(
            |layer_name| CString::new(*layer_name).unwrap()
        ).collect();
        let enabled_layer_names: Vec<*const i8> = required_validation_layer_raw_names.iter().map(
            |layer_name| layer_name.as_ptr()
        ).collect();

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: if ENABLE_VALIDATION_LAYERS {
                &debug_utils_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT as *const c_void
            } else {
                ptr::null()
            },
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &app_info,
            pp_enabled_layer_names: if ENABLE_VALIDATION_LAYERS {
                enabled_layer_names.as_ptr()
            } else {
                ptr::null()
            },
            enabled_layer_count: if ENABLE_VALIDATION_LAYERS {
                enabled_layer_names.len()
            } else {
                0
            } as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
        };

        let instance: ash::Instance = unsafe {
            entry.create_instance(&create_info, None).expect("Failed to create Instance!")
        };

        instance
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: &VkPhysicalDevice,
        surface: &VkSurface
    ) -> (ash::Device, VkQueueFamilyIndices) {
        let indices = VkQueueFamilyIndices::find(instance, physical_device.handle, surface);

        let mut unique_queue_families = HashSet::new();
        unique_queue_families.insert(indices.graphics.unwrap());
        unique_queue_families.insert(indices.present.unwrap());
        unique_queue_families.insert(indices.transfer.unwrap());

        let queue_priorities = [1.0_f32];
        let mut queue_create_infos = vec![];
        for &queue_family in unique_queue_families.iter() {
            queue_create_infos.push(vk::DeviceQueueCreateInfo {
                s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::DeviceQueueCreateFlags::empty(),
                queue_family_index: queue_family,
                p_queue_priorities: queue_priorities.as_ptr(),
                queue_count: queue_priorities.len() as u32,
            });
        }

        let physical_device_features = vk::PhysicalDeviceFeatures {
            sampler_anisotropy: 1,
            ..Default::default()
        };
        let physical_device_buffer_features = vk::PhysicalDeviceBufferDeviceAddressFeatures {
            s_type: vk::StructureType::PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES,
            p_next: std::ptr::null_mut(),
            buffer_device_address: 1,
            buffer_device_address_capture_replay: 0,
            buffer_device_address_multi_device: 0,
            ..Default::default()
        };

        let required_validation_layers_raw_names: Vec<CString> = VALIDATION_LAYERS.iter().map(
            |layer_name| CString::new(*layer_name).unwrap()
        ).collect();

        let enable_validation_layers_names: Vec<*const c_char> = required_validation_layers_raw_names.iter().map(
            |layer_name| layer_name.as_ptr()
        ).collect();

        let device_extensions_raw_names: Vec<CString> = DEVICE_EXTENSIONS.iter().map(
            |extension_name| CString::new(*extension_name).unwrap()
        ).collect();

        let enable_device_extensions_names: Vec<*const c_char> = device_extensions_raw_names.iter().map(
            |extension_name| extension_name.as_ptr()
        ).collect();

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: &physical_device_buffer_features as *const vk::PhysicalDeviceBufferDeviceAddressFeatures as *const c_void,
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_layer_count: if ENABLE_VALIDATION_LAYERS {
                enable_validation_layers_names.len()
            } else {
                0
            } as u32,
            pp_enabled_layer_names: if ENABLE_VALIDATION_LAYERS {
                enable_validation_layers_names.as_ptr()
            } else {
                ptr::null()
            },
            enabled_extension_count: enable_device_extensions_names.len() as u32,
            pp_enabled_extension_names: enable_device_extensions_names.as_ptr(),
            p_enabled_features: &physical_device_features,
        };

        let device: ash::Device = unsafe {
            instance.create_device(physical_device.handle, &device_create_info, None).expect("Failed to create logical Device!")
        };

        (device, indices)
    }
}

impl Drop for VkContext {
    fn drop(&mut self) {
        unsafe {
            self.descriptor_pool.destroy(&self.device);

            self.graphics_command_pool.destroy(&self.device);
            self.transfer_command_pool.destroy(&self.device);

            let mut alloc: Option<Allocator> = None;
            std::mem::swap(&mut alloc, &mut self.allocator);
            drop(alloc);

            self.surface.destroy();
            self.debug_messenger.destroy();
            self.device.destroy_device(None);

            self.instance.destroy_instance(None);
        }
    }
}