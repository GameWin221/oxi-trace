use std::collections::HashSet;


use crate::{
    utilities,
    vk::{
        swapchain::*,
        queue_family_indices::*,
        surface::*
    }
};

pub const DEVICE_EXTENSIONS: [&'static str; 1] = [
    "VK_KHR_swapchain",
];

#[derive(Clone, Debug)]
pub struct VkPhysicalDevice {
    pub handle: ash::vk::PhysicalDevice,
    pub name: String,
    pub queue_family_indices: VkQueueFamilyIndices
}

impl VkPhysicalDevice {
    pub fn new(instance: &ash::Instance, surface: &VkSurface) -> Self {
        let physical_devices: Vec<VkPhysicalDevice> = unsafe {
            instance.enumerate_physical_devices().expect("Failed to enumerate Physical Devices!").into_iter().map(
                |physical_device| Self::from_native(instance, physical_device, surface)
            ).collect()
        };

        println!("Found {} devices (GPU) with vulkan support.", physical_devices.len());

        let result = physical_devices.iter().find(|&physical_device| {
            Self::is_physical_device_suitable(instance, physical_device, surface)
        }).expect("Failed to find a suitable GPU!").clone();

        println!("Picked {} as the vulkan physical device.", result.name);

        result 
    }

    pub fn from_native(instance: &ash::Instance, native_physical_device: ash::vk::PhysicalDevice, surface: &VkSurface) -> Self {
        VkPhysicalDevice { 
            handle: native_physical_device, 
            name: Self::get_physical_device_name(instance, native_physical_device),
            queue_family_indices: VkQueueFamilyIndices::find(instance, native_physical_device, surface)
        }
    }

    fn get_physical_device_name(instance: &ash::Instance, physical_device: ash::vk::PhysicalDevice) -> String {
        let device_properties = unsafe { 
            instance.get_physical_device_properties(physical_device) 
        };
        
        utilities::cchar_to_string(&device_properties.device_name)
    }

    fn is_physical_device_suitable(instance: &ash::Instance, physical_device: &VkPhysicalDevice, surface: &VkSurface) -> bool {
        let device_properties = unsafe { 
            instance.get_physical_device_properties(physical_device.handle) 
        };
        let device_features = unsafe { 
            instance.get_physical_device_features(physical_device.handle) 
        };
        let device_queue_families = unsafe { 
            instance.get_physical_device_queue_family_properties(physical_device.handle)
        };
        let available_extensions = unsafe {
            instance.enumerate_device_extension_properties(physical_device.handle).expect("Failed to get device extension properties.")
        };
        
        Self::print_device_info(
            &device_properties,
            &device_features,
            &device_queue_families,
            &available_extensions
        );

        let indices = VkQueueFamilyIndices::find(instance, physical_device.handle, surface);

        let is_device_extension_supported = Self::query_device_extensions_support(&available_extensions);
        let is_swapchain_supported = if is_device_extension_supported {
            let swapchain_support = VkSwapchain::query_swapchain_support(physical_device, surface);

            !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
        } else {
            false
        };

        return indices.is_complete() && is_device_extension_supported && is_swapchain_supported;
    }

    fn query_device_extensions_support(available_extensions: &Vec<ash::vk::ExtensionProperties>) -> bool {
        let available_extension_names: Vec<String> = available_extensions.iter().map(
            |extension| utilities::cchar_to_string(&extension.extension_name)
        ).collect();

        let mut required_extensions: HashSet<String> = HashSet::from_iter(DEVICE_EXTENSIONS.iter().map(
            |extension| extension.to_string()
        ));

        for extension_name in available_extension_names.iter() {
            required_extensions.remove(extension_name);
        }

        return required_extensions.is_empty();
    }

    #[cfg(debug_assertions)]
    fn print_device_info(
        device_properties: &ash::vk::PhysicalDeviceProperties,
        device_features: &ash::vk::PhysicalDeviceFeatures,
        device_queue_families: &Vec<ash::vk::QueueFamilyProperties>,
        available_extensions: &Vec<ash::vk::ExtensionProperties>,
    ) {
        let device_type = match device_properties.device_type {
            ash::vk::PhysicalDeviceType::CPU => "Cpu",
            ash::vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU",
            ash::vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
            ash::vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
            ash::vk::PhysicalDeviceType::OTHER => "Unknown",
            _ => panic!(),
        };
    
        let device_name = utilities::cchar_to_string(&device_properties.device_name);
        println!("\tDevice Name: {}, id: {}, type: {}", device_name, device_properties.device_id, device_type);
    
        let major_version = ash::vk::api_version_major(device_properties.api_version);
        let minor_version = ash::vk::api_version_minor(device_properties.api_version);
        let patch_version = ash::vk::api_version_patch(device_properties.api_version);
    
        println!("\tAPI Version: {}.{}.{}",major_version, minor_version, patch_version);
    
        println!("\tSupport Queue Family: {}", device_queue_families.len());
        println!("\t\tQueue Count |  Graphics,    Compute,   Transfer,   Sparse Binding");
        
        for queue_family in device_queue_families.iter() {
            let is_graphics_support = if queue_family.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS) {
                " support "
            } else {
                "unsupport"
            };
            let is_compute_support = if queue_family.queue_flags.contains(ash::vk::QueueFlags::COMPUTE) {
                " support "
            } else {
                "unsupport"
            };
            let is_transfer_support = if queue_family.queue_flags.contains(ash::vk::QueueFlags::TRANSFER) {
                " support "
            } else {
                "unsupport"
            };
            let is_sparse_support = if queue_family.queue_flags.contains(ash::vk::QueueFlags::SPARSE_BINDING) {
                " support "
            } else {
                "unsupport"
            };
        
            println!(
                "\t\t{}\t    | {},  {},  {},  {}",
                queue_family.queue_count,
                is_graphics_support,
                is_compute_support,
                is_transfer_support,
                is_sparse_support
            );
        }

        println!("\tAvailable Device Extensions: ");
        for extension in available_extensions.iter() {
            let extension_name = utilities::cchar_to_string(&extension.extension_name);
            println!(
                "\t\tName: {}, Version: {}",
                extension_name, extension.spec_version
            );
        }

        println!("\tAvailable Device Features:\n\t{:?}\n", device_features);
    }

    #[cfg(not(debug_assertions))]
    fn print_device_info(
        device_properties: &ash::vk::PhysicalDeviceProperties,
        device_features: &ash::vk::PhysicalDeviceFeatures,
        device_queue_families: &Vec<ash::vk::QueueFamilyProperties>,
        available_extensions: &Vec<ash::vk::ExtensionProperties>,
    ) {
        
    }

}