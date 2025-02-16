use super::config::RCConfig;
use super::config::RawUniformData;
use egui_wgpu::wgpu;

pub struct RCResources {
    pub uniform_buffer: wgpu::Buffer,

    pub cascade_buffers: [wgpu::Buffer; 2],

    pub sdf_view: wgpu::TextureView,

    // uniform buffer, samplers, sdf texture
    pub uniform_bind_group: wgpu::BindGroup,

    pub in_texture_bgl: wgpu::BindGroupLayout,
    // ith bind group writes to ith buffer
    pub temp_bind_groups: [wgpu::BindGroup; 2],

    pub final_bgl: wgpu::BindGroupLayout,

    pub main_pipeline: wgpu::ComputePipeline,
    pub final_pipeline: wgpu::ComputePipeline,
}

impl RCResources {
    pub const SDF_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
    // cascade buffers store vec2<u32>s
    pub const CASCADE_BUFFER_ELEM_SIZE: u32 = 8;

    fn create_sdf_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rc sdf texture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: RCResources::SDF_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        })
    }

    fn create_cascade_buffers(device: &wgpu::Device, num_elems: u32) -> [wgpu::Buffer; 2] {
        core::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("an rc cascade buffer"),
                size: (num_elems * RCResources::CASCADE_BUFFER_ELEM_SIZE) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        })
    }

    pub fn new(device: &wgpu::Device, window_size: (u32, u32), config: RCConfig) -> Self {
        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rc nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rc linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rc uniform bufer"),
            size: std::mem::size_of::<RawUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cascade_buffers =
            RCResources::create_cascade_buffers(&device, config.get_max_cascade_size(window_size));

        let sdf_texture = RCResources::create_sdf_texture(device, window_size);
        let sdf_view = sdf_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rc uniform bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rc uniform bind group"),
            layout: &uniform_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&sdf_view),
                },
            ],
        });

        let in_texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rc in texture bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let temp_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rc temp bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let temp_bind_groups: [wgpu::BindGroup; 2] = core::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("an rc temp texture bind group"),
                layout: &temp_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            cascade_buffers[1 - i].as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(
                            cascade_buffers[i].as_entire_buffer_binding(),
                        ),
                    },
                ],
            })
        });

        let final_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rc final bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let final_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("final_shader.wgsl"));

        let main_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rc main pipeline layout"),
            bind_group_layouts: &[&uniform_bgl, &in_texture_bgl, &temp_bgl],
            push_constant_ranges: &[],
        });

        let main_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rc main pipeline"),
            layout: Some(&main_pipeline_layout),
            module: &shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let final_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rc final pipeline layout"),
                bind_group_layouts: &[&final_bgl],
                push_constant_ranges: &[],
            });

        let final_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rc final pipeline"),
            layout: Some(&final_pipeline_layout),
            module: &final_shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        RCResources {
            uniform_buffer,

            cascade_buffers,

            sdf_view,

            uniform_bind_group,
            in_texture_bgl,
            temp_bind_groups,
            final_bgl,

            main_pipeline,
            final_pipeline,
        }
    }

    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        in_texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rc textures bind group"),
            layout: &self.in_texture_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(in_texture_view),
            }],
        })
    }

    pub fn create_final_bind_group(
        &self,
        device: &wgpu::Device,
        out_view: &wgpu::TextureView,
        temp_index: usize,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rc final bind group"),
            layout: &self.final_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        self.cascade_buffers[temp_index].as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(out_view),
                },
            ],
        })
    }
}
