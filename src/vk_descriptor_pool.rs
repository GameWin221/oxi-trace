use ash::vk;

const DESCRIPTOR_POOL_SIZES_COUNT: u32 = 64; 

#[derive(Clone, Copy, Debug, Default)]
pub struct VkDescriptorSet {
    pub handle: vk::DescriptorSet,
    pub layout: vk::DescriptorSetLayout
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VkDescriptorPool {
    pub handle: vk::DescriptorPool,
}

pub struct VkDescriptorSetSlot {
    pub binding: vk::DescriptorSetLayoutBinding,
    pub buffer_info: Option<vk::DescriptorBufferInfo>,
    pub image_info: Option<vk::DescriptorImageInfo>,
}

impl VkDescriptorPool {
    pub fn new(device: &ash::Device) -> Self {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
        ];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET) 
            .max_sets(pool_sizes.len() as u32 * DESCRIPTOR_POOL_SIZES_COUNT)
            .pool_sizes(&pool_sizes)
            .build();

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&descriptor_pool_create_info, None).expect("Failed to create Descriptor pool!")
        };

        Self {
            handle: descriptor_pool
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.handle, None);
        }
    }

    pub fn allocate(&self, device: &ash::Device, slots: &Vec<VkDescriptorSetSlot>) -> VkDescriptorSet {
        let layout_bindings: Vec<vk::DescriptorSetLayoutBinding> = slots.iter().map(
            |slot| slot.binding
        ).collect();

        let layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&layout_bindings)
            .build();

        let layout = unsafe {
            device.create_descriptor_set_layout(&layout_create_info, None).expect("Failed to create Descriptor Set Layout!")
        };

        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.handle)
            .set_layouts(&[layout])
            .build();

        let descriptor_set = unsafe {
            device.allocate_descriptor_sets(&allocate_info).expect("Failed to allocate a descriptor set!")[0]
        };

        let mut descriptor_write_sets = vec![];
        for slot in slots {
            let info = if let Some(image_info) = slot.image_info {
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .descriptor_type(slot.binding.descriptor_type)
                    .dst_binding(slot.binding.binding)
                    .image_info(&[image_info])
                    .build()
            } else if let Some(buffer_info) = slot.buffer_info {
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .descriptor_type(slot.binding.descriptor_type)
                    .dst_binding(slot.binding.binding)
                    .buffer_info(&[buffer_info])
                    .build()
            } else {
                panic!("image_info and buffer_info are both None!")
            };

            descriptor_write_sets.push(info);
        }

        unsafe {
            device.update_descriptor_sets(&descriptor_write_sets, &[]);
        }

        VkDescriptorSet { 
            handle: descriptor_set, 
            layout 
        }
    }

    pub fn deallocate(&self, device: &ash::Device, target: &VkDescriptorSet) {
        unsafe {
            device.free_descriptor_sets(self.handle, &[target.handle]).expect("Failed to free descriptor sets!");
            device.destroy_descriptor_set_layout(target.layout, None);
        }
    }
}