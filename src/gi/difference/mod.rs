use super::GIRenderer;
use egui_wgpu::wgpu;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub enum DiffMode {
    Abs = 0,
    FirstMinusSecond = 1,
    SecondMinusFirst = 2,
}

pub struct DiffConfig {
    pub mode: DiffMode,
    pub mult: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct RawUniformData {
    mode: u32,
    mult: f32,
}

pub struct Difference {
    pub config: DiffConfig,

    temp_textures: [wgpu::Texture; 2],

    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    out_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl Difference {
    fn create_temp_textures(device: &wgpu::Device, size: (u32, u32)) -> [wgpu::Texture; 2] {
        let ct = || {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("a diff temp texture"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            })
        };

        [ct(), ct()]
    }

    pub fn new(device: &wgpu::Device, texture_size: (u32, u32)) -> Self {
        let temp_textures = Difference::create_temp_textures(device, texture_size);
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("diff uniform buffer"),
            size: std::mem::size_of::<RawUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("diff bind group layout"),
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diff bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &temp_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &temp_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        let out_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("diff out bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("diff pipeline layout"),
            bind_group_layouts: &[&bind_group_layout, &out_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("diff compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Difference {
            config: DiffConfig {
                mode: DiffMode::Abs,
                mult: 1.,
            },
            temp_textures,

            uniform_buffer,
            bind_group,
            out_bind_group_layout,

            pipeline,
        }
    }

    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, out_texture: &wgpu::Texture) {
        let out_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("a diff out bind group"),
            layout: &self.out_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&RawUniformData {
                mode: self.config.mode as u32,
                mult: self.config.mult.into(),
            }),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("diff compute pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.set_bind_group(1, &out_bind_group, &[]);
            // TODO
            compute_pass.dispatch_workgroups(
                (out_texture.size().width + 15) / 16,
                (out_texture.size().height + 15) / 16,
                1,
            );
        }

        queue.submit(Some(encoder.finish()));
    }

    pub fn textures(&self) -> &[wgpu::Texture] {
        &self.temp_textures
    }
}
