

use crate::utilities;

use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

pub const VALIDATION_LAYERS: [&'static str; 1] = [
    "VK_LAYER_KHRONOS_validation"
];

#[cfg(not(debug_assertions))]
pub const ENABLE_VALIDATION_LAYERS: bool = false;
#[cfg(debug_assertions)]
pub const ENABLE_VALIDATION_LAYERS: bool = true;

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> ash::vk::Bool32 {
    let severity = match message_severity {
        ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message = CStr::from_ptr((*p_callback_data).p_message);
    println!("[Debug]{}{}{:?}", severity, types, message);

    ash::vk::FALSE
}

pub fn check_validation_layer_support(entry: &ash::Entry) -> bool {
    let layer_properties = entry.enumerate_instance_layer_properties().expect("Failed to enumerate Instance Layers Properties!");

    if layer_properties.len() <= 0 {
        eprintln!("No available layers.");
        return false;
    }

    for required_layer_name in VALIDATION_LAYERS.iter() {
        let mut is_layer_found = false;

        for layer_property in layer_properties.iter() {
            let test_layer_name = utilities::cchar_to_string(&layer_property.layer_name);
            if (*required_layer_name) == test_layer_name {
                is_layer_found = true;
                break;
            }
        }

        if is_layer_found == false {
            return false;
        }
    }

    true
}

pub fn populate_debug_messenger_create_info() -> ash::vk::DebugUtilsMessengerCreateInfoEXT {
    ash::vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: ash::vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: ash::vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            // ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
            // ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        p_user_data: ptr::null_mut(),
    }
}

#[derive(Clone)]
pub struct VkDebugMessenger {
    debug_utils: ash::extensions::ext::DebugUtils,
    messenger: ash::vk::DebugUtilsMessengerEXT
}

impl VkDebugMessenger {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let debug_utils = ash::extensions::ext::DebugUtils::new(entry, instance);

        if ENABLE_VALIDATION_LAYERS == false {
            Self {
                debug_utils,
                messenger: ash::vk::DebugUtilsMessengerEXT::null()
            }
        } else {
            let messenger_ci = self::populate_debug_messenger_create_info();

            let messenger = unsafe {
                debug_utils.create_debug_utils_messenger(&messenger_ci, None).expect("Debug Utils Callback")
            };
            Self {
                debug_utils, 
                messenger
            }
        }
    }

    pub fn destroy(&self) {
        if ENABLE_VALIDATION_LAYERS {
            unsafe {
                self.debug_utils.destroy_debug_utils_messenger(self.messenger, None);
            }
        }
    }
}