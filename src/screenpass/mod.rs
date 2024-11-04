use egui_wgpu::wgpu;

/// struct representing a 2d screen/texture quad pass
///
/// intended to be used as a field in a struct representing a specific pass
/// that struct is the one that holds data like buffers and textures, not this one
pub struct ScreenPass {
    shader_module: wgpu::ShaderModule,
    bind_group_layouts: Box<[wgpu::BindGroupLayout]>,
    pipeline_layout: wgpu::PipelineLayout,
}

impl ScreenPass {
    pub fn new(
        device: &wgpu::Device,
        //bind_group_layout_binding_types: Vec<Vec<wgpu::BindingType>>,
        bind_group_layout_binding_types: &[&[wgpu::BindingType]],
        shader_module: wgpu::ShaderModule,
    ) -> Self {
        let bind_group_layouts: Box<[wgpu::BindGroupLayout]> = {
            let all_entries: Vec<Vec<wgpu::BindGroupLayoutEntry>> = bind_group_layout_binding_types
                .iter()
                .map(|tys| {
                    tys.iter()
                        .enumerate()
                        .map(|(i, ty)| wgpu::BindGroupLayoutEntry {
                            binding: i as u32,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: *ty,
                            count: None,
                        })
                        .collect()
                })
                .collect();

            all_entries
                .iter()
                .map(|entries| {
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("a screenpass bind group layout"),
                        entries,
                    })
                })
                .collect()
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("a screenpass pipeline layout"),
            bind_group_layouts: bind_group_layouts
                .iter()
                .collect::<Box<[&wgpu::BindGroupLayout]>>()
                .as_ref(),
            push_constant_ranges: &[],
        });

        Self {
            shader_module,
            bind_group_layouts,
            pipeline_layout,
        }
    }

    fn create_bind_groups(
        &self,
        device: &wgpu::Device,
        // must be in the same order as bind group layouts
        //resources: Vec<Vec<wgpu::BindingResource>>,
        resources: &[&[wgpu::BindingResource]],
    ) -> Box<[wgpu::BindGroup]> {
        resources
            .iter()
            .enumerate()
            .map(|(bgi, res)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("a screenpass bind group"),
                    layout: &self.bind_group_layouts[bgi],
                    entries: res
                        .iter()
                        .enumerate()
                        .map(|(binding, resource)| wgpu::BindGroupEntry {
                            binding: binding as u32,
                            resource: resource.clone(),
                        })
                        .collect::<Vec<_>>()
                        .as_ref(),
                })
            })
            .collect()
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        fragment_targets: &[Option<wgpu::ColorTargetState>],
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("a screenpass render pipeline"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader_module,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader_module,
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

    pub fn render(&self, desc: &ScreenPassRenderDescriptor) {
        let render_pipeline = self.create_pipeline(desc.device, desc.fragment_targets);
        let bind_groups = self.create_bind_groups(desc.device, desc.bind_group_resources);

        let mut encoder = desc
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("a screenpass render pass"),
                color_attachments: desc.color_attachments,
                ..Default::default()
            });

            render_pass.set_pipeline(&render_pipeline);
            bind_groups
                .iter()
                .enumerate()
                .for_each(|(i, bg)| render_pass.set_bind_group(i as u32, bg, &[]));
            render_pass.draw(0..4, 0..1);
        }

        desc.queue.submit(Some(encoder.finish()));
    }
}

// TODO: is this even a good idea
pub struct ScreenPassRenderDescriptor<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub fragment_targets: &'a [Option<wgpu::ColorTargetState>],
    pub bind_group_resources: &'a [&'a [wgpu::BindingResource<'a>]],
    pub color_attachments: &'a [Option<wgpu::RenderPassColorAttachment<'a>>],
}
