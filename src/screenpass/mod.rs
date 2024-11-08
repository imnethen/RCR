use egui_wgpu::wgpu;

/// struct representing a 2d screen/texture quad pass
///
/// intended to be used as a field in a struct representing a specific pass
/// that struct is the one that holds data like buffers and textures, not this one
pub struct ScreenPass {
    label: String,
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl ScreenPass {
    fn create_pipeline(
        device: &wgpu::Device,
        label: &str,
        bind_group_layout: &wgpu::BindGroupLayout,
        shader_module: &wgpu::ShaderModule,
        fragment_targets: &[Option<wgpu::ColorTargetState>],
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&[label, "render pipeline"].join("")),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: fragment_targets,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multiview: None,
            multisample: Default::default(),
        })
    }

    pub fn new(
        device: &wgpu::Device,
        label: Option<&str>,
        bind_group_layout_binding_types: &[wgpu::BindingType],
        shader_module: wgpu::ShaderModule,
        fragment_targets: &[Option<wgpu::ColorTargetState>],
    ) -> Self {
        let label = label.unwrap_or("").to_owned();

        let bind_group_layout: wgpu::BindGroupLayout = {
            let entries = bind_group_layout_binding_types
                .iter()
                .enumerate()
                .map(|(i, ty)| wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: *ty,
                    count: None,
                })
                .collect::<Vec<wgpu::BindGroupLayoutEntry>>();

            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&(label.clone() + " bind group layout")),
                entries: entries.as_slice(),
            })
        };

        let render_pipeline = ScreenPass::create_pipeline(
            device,
            &label,
            &bind_group_layout,
            &shader_module,
            fragment_targets,
        );

        Self {
            label,
            bind_group_layout,
            render_pipeline,
        }
    }

    fn create_bind_groups(
        &self,
        device: &wgpu::Device,
        // must be in the same order as bind group layouts
        resources: &[wgpu::BindingResource],
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&(self.label.clone() + " bind group")),
            layout: &self.bind_group_layout,
            entries: resources
                .iter()
                .enumerate()
                .map(|(binding, resource)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: resource.clone(),
                })
                .collect::<Vec<_>>()
                .as_ref(),
        })
    }

    pub fn render(&self, desc: &ScreenPassRenderDescriptor) {
        let bind_group = self.create_bind_groups(desc.device, desc.bind_group_resources);

        let mut encoder = desc
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&(self.label.clone() + " render pass")),
                color_attachments: desc.color_attachments,
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }

        desc.queue.submit(Some(encoder.finish()));
    }
}

// TODO: is this even a good idea
pub struct ScreenPassRenderDescriptor<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub bind_group_resources: &'a [wgpu::BindingResource<'a>],
    pub color_attachments: &'a [Option<wgpu::RenderPassColorAttachment<'a>>],
}
