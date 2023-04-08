use std::ffi::CStr;
use std::os::raw::c_char;
use ash::vk;

use crate::{
    vk_command_pool::{VkCommandPool, VkCommandBuffer},
    vk_queue::VkQueue
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

pub fn begin_single_queue_submit(device: &ash::Device, command_pool: &VkCommandPool) -> VkCommandBuffer
{
    let cmd = command_pool.allocate(device, 1)[0];
    cmd.begin_recording(device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    cmd
}
pub fn end_single_queue_submit(device: &ash::Device, command_pool: &VkCommandPool, queue: &VkQueue, command_buffer: VkCommandBuffer)
{
    command_buffer.end_recording(device);
    queue.submit_once(device, &command_buffer);
    command_pool.deallocate(device, &command_buffer);
}