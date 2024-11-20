use super::GIRenderer;
use crate::jfa::JFA;
use egui_wgpu::wgpu;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct RawUniformData {
    ray_count: u32,
}

struct RaymarcherConfig {
    ray_count: u32,
}

pub struct Raymarcher {
    config: RaymarcherConfig,
    window_size: (u32, u32),

    uniform_buffer: wgpu::Buffer,
    sdf_texture: wgpu::Texture,
    sdf_view: wgpu::TextureView,

    jfa: JFA,

    uniform_bind_group: wgpu::BindGroup,

    in_texture_bind_group_layout: wgpu::BindGroupLayout,
    out_texture_bind_group_layout: wgpu::BindGroupLayout,

    pipeline: wgpu::ComputePipeline,
}

impl Raymarcher {
    pub fn new(
        device: &wgpu::Device,
        window_size: (u32, u32),
        out_texture_format: wgpu::TextureFormat,
    ) -> Self {
        // TODO
        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("raymarcher nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("raymarcher linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raymarcher uniform bufer"),
            size: std::mem::size_of::<RawUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sdf_format = wgpu::TextureFormat::R32Float;

        let sdf_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raymarcher sdf texture"),
            size: wgpu::Extent3d {
                width: window_size.0,
                height: window_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: sdf_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let sdf_view = sdf_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let jfa = JFA::new(device, window_size, sdf_format);

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("raymarcher uniform bind group layout"),
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raymarcher uniform bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&sdf_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        let in_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("raymarcher texture bind group layout"),
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

        let out_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("raymarcher out texture bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: out_texture_format,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("texture renderer pipeline layout"),
            bind_group_layouts: &[
                &uniform_bind_group_layout,
                &in_texture_bind_group_layout,
                &out_texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("raymarcher pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Raymarcher {
            config: RaymarcherConfig { ray_count: 1024 },
            window_size,

            uniform_buffer,
            sdf_texture,
            sdf_view,
            uniform_bind_group,

            jfa,

            in_texture_bind_group_layout,
            out_texture_bind_group_layout,

            pipeline,
        }
    }

    fn create_texture_bind_groups(
        &self,
        device: &wgpu::Device,
        in_texture_view: &wgpu::TextureView,
        out_texture_view: &wgpu::TextureView,
    ) -> (wgpu::BindGroup, wgpu::BindGroup) {
        (
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raymarcher in texture bind group"),
                layout: &self.in_texture_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(in_texture_view),
                }],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raymarcher out texture bind group"),
                layout: &self.out_texture_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(out_texture_view),
                }],
            }),
        )
    }
}

impl GIRenderer for Raymarcher {
    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        let uniform_data = RawUniformData {
            ray_count: self.config.ray_count,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

        let in_view = in_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let out_view = out_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (in_texture_bind_group, out_texture_bind_group) =
            self.create_texture_bind_groups(device, &in_view, &out_view);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.jfa
            .render(device, queue, in_texture, &self.sdf_texture);

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, &in_texture_bind_group, &[]);
            compute_pass.set_bind_group(2, &out_texture_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                (self.window_size.0 + 15) / 16,
                (self.window_size.1 + 15) / 16,
                1,
            );
        }

        queue.submit(Some(encoder.finish()));
    }

    fn render_egui(&mut self, ctx: &egui::Context) {
        egui::Window::new("raymarcher").show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.config.ray_count, 4..=8196).logarithmic(true));
        });
    }
}
