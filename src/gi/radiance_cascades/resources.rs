use super::config::RCConfig;
use super::config::RawUniformData;
use egui_wgpu::wgpu;

pub struct RCResources {
    pub uniform_buffer: wgpu::Buffer,

    pub temp_textures: [wgpu::Texture; 2],
    pub temp_views: [wgpu::TextureView; 2],

    pub sdf_texture: wgpu::Texture,
    pub sdf_view: wgpu::TextureView,

    // uniform buffer, samplers, sdf texture
    pub uniform_bind_group: wgpu::BindGroup,

    pub in_texture_bgl: wgpu::BindGroupLayout,
    // ith bind group writes to ith texture
    pub temp_bind_groups: [wgpu::BindGroup; 2],

    pub final_bgl: wgpu::BindGroupLayout,

    pub main_pipeline: wgpu::ComputePipeline,
    pub final_pipeline: wgpu::ComputePipeline,
}

impl RCResources {
    pub const SDF_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
    pub const TEMP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

    fn cascade_size_to_extent(cascade_size: u32, window_size: (u32, u32)) -> wgpu::Extent3d {
        let width = window_size.0;
        let height = cascade_size.div_ceil(width);
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        }
    }

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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn create_temp_textures(device: &wgpu::Device, size: wgpu::Extent3d) -> [wgpu::Texture; 2] {
        core::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("an rc temp texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: RCResources::TEMP_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
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

        let temp_textures = RCResources::create_temp_textures(
            device,
            RCResources::cascade_size_to_extent(
                config.get_max_cascade_size(window_size),
                window_size,
            ),
        );
        let temp_views = [
            temp_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            temp_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];

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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: RCResources::TEMP_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
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
                        resource: wgpu::BindingResource::TextureView(&temp_views[1 - i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&temp_views[i]),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
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

            temp_textures,
            temp_views,

            sdf_texture,
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
                    resource: wgpu::BindingResource::TextureView(&self.temp_views[temp_index]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(out_view),
                },
            ],
        })
    }
}
