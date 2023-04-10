

const DESCRIPTOR_POOL_SIZES_COUNT: u32 = 64; 

#[derive(Clone, Copy, Debug, Default)]
pub struct VkDescriptorSet {
    pub handle: ash::vk::DescriptorSet,
    pub layout: ash::vk::DescriptorSetLayout
}

impl VkDescriptorSet {
    pub fn update(&self, device: &ash::Device, slots: &Vec<VkDescriptorSetSlot>) {
        let mut descriptor_write_sets = vec![];
        for slot in slots {
            let info = if let Some(image_info) = slot.image_info {
                ash::vk::WriteDescriptorSet::builder()
                    .dst_set(self.handle)
                    .descriptor_type(slot.binding.descriptor_type)
                    .dst_binding(slot.binding.binding)
                    .image_info(&[image_info])
                    .build()
            } else if let Some(buffer_info) = slot.buffer_info {
                ash::vk::WriteDescriptorSet::builder()
                    .dst_set(self.handle)
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
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VkDescriptorPool {
    pub handle: ash::vk::DescriptorPool,
}

pub struct VkDescriptorSetSlot {
    pub binding: ash::vk::DescriptorSetLayoutBinding,
    pub buffer_info: Option<ash::vk::DescriptorBufferInfo>,
    pub image_info: Option<ash::vk::DescriptorImageInfo>,
}

impl VkDescriptorPool {
    pub fn new(device: &ash::Device) -> Self {
        let pool_sizes = [
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::SAMPLER,
                descriptor_count: DESCRIPTOR_POOL_SIZES_COUNT,
            },
        ];

        let descriptor_pool_create_info = ash::vk::DescriptorPoolCreateInfo::builder()
            .flags(ash::vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET) 
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
        let layout_bindings: Vec<ash::vk::DescriptorSetLayoutBinding> = slots.iter().map(
            |slot| slot.binding
        ).collect();

        let layout_create_info = ash::vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&layout_bindings)
            .build();

        let layout = unsafe {
            device.create_descriptor_set_layout(&layout_create_info, None).expect("Failed to create Descriptor Set Layout!")
        };

        let allocate_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.handle)
            .set_layouts(&[layout])
            .build();

        let handle = unsafe {
            device.allocate_descriptor_sets(&allocate_info).expect("Failed to allocate a descriptor set!")[0]
        };

        let descriptor_set = VkDescriptorSet { handle, layout };

        descriptor_set.update(device, slots);

        descriptor_set
    }

    pub fn deallocate(&self, device: &ash::Device, target: &VkDescriptorSet) {
        unsafe {
            device.free_descriptor_sets(self.handle, &[target.handle]).expect("Failed to free descriptor sets!");
            device.destroy_descriptor_set_layout(target.layout, None);
        }
    }
}