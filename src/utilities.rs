use std::ffi::CStr;
use std::os::raw::c_char;


use crate::vk::{
    command_pool::VkCommandPool,
    command_buffer::VkCommandBuffer,
    queue::VkQueue
};

pub fn cchar_to_string(raw_string_array: &[c_char]) -> String {  
    let raw_string = unsafe {
        CStr::from_ptr(raw_string_array.as_ptr())
    };

    raw_string.to_str().expect("Failed to convert const char raw string").to_owned()
}

pub fn read_spirv(shader_path: &std::path::Path) -> Vec<u8> {
    use std::fs::File;
    use std::io::Read;

    let spv_file = File::open(shader_path).expect(&format!("Failed to find spv file at {:?}", shader_path));

    spv_file.bytes().filter_map(|byte| byte.ok()).collect()
}
pub fn create_shader_module(device: &ash::Device, spirv: &Vec<u8>) -> ash::vk::ShaderModule {
    let shader_module_create_info = ash::vk::ShaderModuleCreateInfo {
        s_type: ash::vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: ash::vk::ShaderModuleCreateFlags::empty(),
        code_size: spirv.len(),
        p_code: spirv.as_ptr() as *const u32,
    };

    unsafe {
        device.create_shader_module(&shader_module_create_info, None).expect("Failed to create Shader Module!")
    }
}

pub fn begin_single_queue_submit(device: &ash::Device, command_pool: &VkCommandPool) -> VkCommandBuffer
{
    let cmd = command_pool.allocate(device, 1)[0];
    cmd.begin_recording(device, ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    cmd
}
pub fn end_single_queue_submit(device: &ash::Device, command_pool: &VkCommandPool, queue: &VkQueue, command_buffer: VkCommandBuffer)
{
    command_buffer.end_recording(device);
    queue.submit_once(device, &command_buffer);
    command_pool.deallocate(device, &command_buffer);
}