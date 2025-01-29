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
    pub label: String,

    config: RaymarcherConfig,
    window_size: (u32, u32),

    uniform_buffer: wgpu::Buffer,
    sdf_texture: wgpu::Texture,
    sdf_view: wgpu::TextureView,

    jfa: JFA,

    uniform_bind_group: wgpu::BindGroup,

    // sdf, in, out
    textures_bgl: wgpu::BindGroupLayout,

    pipeline: wgpu::ComputePipeline,
}

impl Raymarcher {
    const SDF_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

    fn create_sdf_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raymarcher sdf texture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Raymarcher::SDF_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    pub fn new(
        device: &wgpu::Device,
        window_size: (u32, u32),
        out_texture_format: wgpu::TextureFormat,
        label: String,
    ) -> Self {
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

        let sdf_texture = Raymarcher::create_sdf_texture(device, window_size);
        let sdf_view = sdf_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let jfa = JFA::new(device, window_size, Raymarcher::SDF_FORMAT);

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raymarcher uniform bind group"),
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
            ],
        });

        let textures_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raymarcher texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: out_texture_format,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("texture renderer pipeline layout"),
            bind_group_layouts: &[&uniform_bgl, &textures_bgl],
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
            label,

            config: RaymarcherConfig { ray_count: 64 },
            window_size,

            uniform_buffer,
            sdf_texture,
            sdf_view,
            uniform_bind_group,

            jfa,

            textures_bgl,

            pipeline,
        }
    }

    fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        in_texture_view: &wgpu::TextureView,
        out_texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raymarcher textures bind group"),
            layout: &self.textures_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.sdf_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(in_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(out_texture_view),
                },
            ],
        })
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

        self.jfa
            .render(device, queue, in_texture, &self.sdf_texture);

        let textures_bind_group = self.create_texture_bind_group(device, &in_view, &out_view);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, &textures_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                u32::div_ceil(self.window_size.0, 16),
                u32::div_ceil(self.window_size.1, 16),
                1,
            );
        }

        queue.submit(Some(encoder.finish()));
    }

    fn render_egui(&mut self, ctx: &egui::Context, _: &wgpu::Device) {
        egui::Window::new(&self.label).show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.config.ray_count, 4..=8196).logarithmic(true));
        });
    }

    fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        self.window_size = new_size;
        self.sdf_texture = Raymarcher::create_sdf_texture(device, new_size);
        self.sdf_view = self
            .sdf_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.jfa = JFA::new(device, new_size, Raymarcher::SDF_FORMAT);
    }
}
