use egui_wgpu::wgpu;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct RawUniformData {
    color: [f32; 3],
    shape: u32,
    pos: [u32; 2],
    radius: f32,
    _pad: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BrushShape {
    Square = 0,
    Circle = 1,
}

/// can only be used fo Rgba8Unorm textures
pub struct Brush {
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    out_texture_bgl: wgpu::BindGroupLayout,

    pipeline: wgpu::ComputePipeline,
}

impl Brush {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Brush uniform buffer"),
            size: std::mem::size_of::<RawUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brush uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brush uniform bind group"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let out_texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brush out texture bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("brush pipeline layout"),
            bind_group_layouts: &[&uniform_bgl, &out_texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("brush pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Self {
            uniform_buffer,
            uniform_bind_group,

            out_texture_bgl,

            pipeline,
        }
    }

    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        out_texture_view: &wgpu::TextureView,
        shape: BrushShape,
        pos: [u32; 2],
        size: u32,
        color: [f32; 3],
    ) {
        let uniform_data = RawUniformData {
            color,
            shape: shape as u32,
            pos,
            radius: size as f32 / 2.,
            _pad: 0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

        let out_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brush out texture bind group"),
            layout: &self.out_texture_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(out_texture_view),
            }],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, &out_texture_bind_group, &[]);
            compute_pass.dispatch_workgroups(size, size, 1);
        }

        queue.submit(Some(encoder.finish()));
    }
}
