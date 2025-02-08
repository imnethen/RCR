use egui_wgpu::wgpu;

pub struct JFA {
    in_texture_bgl: wgpu::BindGroupLayout,
    prepare_pipeline: wgpu::ComputePipeline,

    main_bind_groups: [wgpu::BindGroup; 2],
    main_pipeline: wgpu::ComputePipeline,

    out_texture_bgl: wgpu::BindGroupLayout,
    final_pipeline: wgpu::ComputePipeline,
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

    pub fn new(device: &wgpu::Device, window_size: (u32, u32)) -> Self {
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

        let in_texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("jfa in texture bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let main_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("jfa main bgl"),
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
                layout: &main_bgl,
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

        let out_texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("jfa out texture bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::R16Float,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            }],
        });

        let prepare_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("jfa prepare pipeline layout"),
                bind_group_layouts: &[&in_texture_bgl, &main_bgl],
                push_constant_ranges: &[],
            });

        let prepare_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("jfa prepare pipeline"),
            layout: Some(&prepare_pipeline_layout),
            module: &prepare_shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let main_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("jfa main pipeline layout"),
            bind_group_layouts: &[&main_bgl],
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

        let final_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("jfa final pipeline layout"),
                bind_group_layouts: &[&main_bgl, &out_texture_bgl],
                push_constant_ranges: &[],
            });

        let final_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("jfa final pipeline"),
            layout: Some(&final_pipeline_layout),
            module: &final_shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        JFA {
            in_texture_bgl,
            prepare_pipeline,

            main_bind_groups,
            main_pipeline,

            out_texture_bgl,
            final_pipeline,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture_view: &wgpu::TextureView,
        out_texture_view: &wgpu::TextureView,
        // TODO: not need the copy
        texture_size: impl Into<(u32, u32)> + Copy,
    ) {
        let num_workgroups = {
            let (w, h) = texture_size.into();
            (u32::div_ceil(w, 16), u32::div_ceil(h, 16))
        };

        let in_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("jfa in texture bind group"),
            layout: &self.in_texture_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&in_texture_view),
            }],
        });

        let out_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("jfa out texture bind group"),
            layout: &self.out_texture_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&out_texture_view),
            }],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut prepare_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            prepare_pass.set_pipeline(&self.prepare_pipeline);
            prepare_pass.set_bind_group(0, &in_texture_bind_group, &[]);
            prepare_pass.set_bind_group(1, &self.main_bind_groups[0], &[]);
            prepare_pass.dispatch_workgroups(num_workgroups.0, num_workgroups.1, 1);
        }

        let mut stepsize: u32 = {
            let (w, h) = texture_size.into();
            f32::sqrt((w * w + h * h) as f32) as u32
        };

        for i in 1.. {
            let mut main_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            main_pass.set_pipeline(&self.main_pipeline);
            main_pass.set_bind_group(0, &self.main_bind_groups[i % 2], &[]);
            main_pass.set_push_constants(0, &stepsize.to_le_bytes());
            main_pass.dispatch_workgroups(num_workgroups.0, num_workgroups.1, 1);

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

        {
            let mut final_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            final_pass.set_pipeline(&self.final_pipeline);
            final_pass.set_bind_group(0, &self.main_bind_groups[1], &[]);
            final_pass.set_bind_group(1, &out_texture_bind_group, &[]);
            final_pass.dispatch_workgroups(num_workgroups.0, num_workgroups.1, 1);
        }

        queue.submit(Some(encoder.finish()));
    }
}
