use crate::screenpass::{self, ScreenPass};
use egui_wgpu::wgpu;

pub struct JFA {
    prepare_pass: ScreenPass,

    temp_textures: [wgpu::Texture; 2],
    main_bind_groups: [wgpu::BindGroup; 2],
    main_pipeline: wgpu::ComputePipeline,

    final_pass: ScreenPass,
}

impl JFA {
    const TEMP_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

    fn create_temp_textures(device: &wgpu::Device, window_size: (u32, u32)) -> [wgpu::Texture; 2] {
        core::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("a jfa temp texture"),
                size: wgpu::Extent3d {
                    width: window_size.0,
                    height: window_size.1,
                    depth_or_array_layers: 1,
                },
                format: JFA::TEMP_TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            })
        })
    }

    pub fn new(
        device: &wgpu::Device,
        window_size: (u32, u32),
        out_texture_format: wgpu::TextureFormat,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("jfa smapler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let temp_textures = JFA::create_temp_textures(device, window_size);
        let temp_texture_views = temp_textures
            .iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect::<Vec<wgpu::TextureView>>();

        let prepare_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("prepare.wgsl"));
        let main_shader_module = device.create_shader_module(wgpu::include_wgsl!("main.wgsl"));
        let final_shader_module = device.create_shader_module(wgpu::include_wgsl!("final.wgsl"));

        let prepare_bind_group_binding_types = &[wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }];

        let prepare_pass = ScreenPass::new(
            device,
            Some("JFA prepare pass"),
            prepare_bind_group_binding_types,
            prepare_shader_module,
            &[Some(wgpu::ColorTargetState {
                format: JFA::TEMP_TEXTURE_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        );

        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("jfa main bind group layout"),
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
                            format: JFA::TEMP_TEXTURE_FORMAT,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let main_bind_groups: [wgpu::BindGroup; 2] = core::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("a jfa main bind group"),
                layout: &main_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&temp_texture_views[1 - i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&temp_texture_views[i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        });

        let main_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("jfa main pipeline layout"),
            bind_group_layouts: &[&main_bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range: 0..4,
            }],
        });

        let main_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("jfa main compute pipeline"),
            layout: Some(&main_pipeline_layout),
            module: &main_shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let final_bind_group_binding_types = &[wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }];

        let final_pass = ScreenPass::new(
            device,
            Some("JFA final pass"),
            final_bind_group_binding_types,
            final_shader_module,
            &[Some(wgpu::ColorTargetState {
                format: out_texture_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        );

        JFA {
            prepare_pass,

            temp_textures,
            main_bind_groups,
            main_pipeline,

            final_pass,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        self.prepare_pass
            .render(&screenpass::ScreenPassRenderDescriptor {
                device,
                queue,
                bind_group_resources: &[wgpu::BindingResource::TextureView(
                    &in_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                )],
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.temp_textures[0]
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations::default(),
                })],
            });

        let mut stepsize: u32 = {
            let w = in_texture.size().width as f32;
            let h = in_texture.size().height as f32;

            f32::sqrt(w * w + h * h) as u32
        };

        let mut main_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        for i in 1.. {
            let mut main_pass =
                main_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            main_pass.set_pipeline(&self.main_pipeline);
            main_pass.set_bind_group(0, &self.main_bind_groups[i % 2], &[]);
            main_pass.set_push_constants(0, &stepsize.to_le_bytes());
            main_pass.dispatch_workgroups(
                u32::div_ceil(in_texture.size().width, 16),
                u32::div_ceil(in_texture.size().height, 16),
                1,
            );

            stepsize /= 2;
            // TODO: make a nonhacky way to end on temp_textures[0]
            // or just dont do that at all i dont need it why am i doing this
            if stepsize == 0 {
                if i % 2 == 1 {
                    stepsize = 1;
                } else {
                    break;
                }
            }
        }

        queue.submit(Some(main_encoder.finish()));

        self.final_pass
            .render(&screenpass::ScreenPassRenderDescriptor {
                device,
                queue,
                bind_group_resources: &[wgpu::BindingResource::TextureView(
                    &self.temp_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
                )],
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations::default(),
                })],
            });
    }
}
