use crate::screenpass::{self, ScreenPass};
use egui_wgpu::wgpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct MainPassRawUniformData {
    step: u32,
}

pub struct JFA {
    prepare_pass: ScreenPass,

    temp_textures: [wgpu::Texture; 2],
    main_uniform_buffer: wgpu::Buffer,
    main_pass: ScreenPass,

    final_pass: ScreenPass,
}

impl JFA {
    fn create_temp_textures(device: &wgpu::Device, window_size: (u32, u32)) -> [wgpu::Texture; 2] {
        let ct = || {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("a jfa temp texture"),
                size: wgpu::Extent3d {
                    width: window_size.0,
                    height: window_size.1,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Rg16Sint,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            })
        };

        [ct(), ct()]
    }

    pub fn new(
        device: &wgpu::Device,
        window_size: (u32, u32),
        out_texture_format: wgpu::TextureFormat,
    ) -> Self {
        let prepare_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("prepare.wgsl"));
        let main_shader_module = device.create_shader_module(wgpu::include_wgsl!("main.wgsl"));
        let final_shader_module = device.create_shader_module(wgpu::include_wgsl!("final.wgsl"));

        let prepare_bind_group_binding_types = &[wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }];

        let prepare_pass = ScreenPass::new(
            device,
            Some("JFA prepare pass"),
            prepare_bind_group_binding_types,
            prepare_shader_module,
            &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rg16Sint,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        );

        let main_bind_group_binding_types = &[
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Sint,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
        ];

        let main_pass = ScreenPass::new(
            device,
            Some("JFA main pass"),
            main_bind_group_binding_types,
            main_shader_module,
            &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rg16Sint,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        );

        let final_bind_group_binding_types = &[wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Sint,
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

        let main_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("jfa main pass uniform buffer"),
            size: std::mem::size_of::<MainPassRawUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let temp_textures = JFA::create_temp_textures(device, window_size);

        JFA {
            prepare_pass,

            temp_textures,
            main_uniform_buffer,
            main_pass,

            final_pass,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_window_size: (u32, u32)) {
        self.temp_textures = JFA::create_temp_textures(device, new_window_size);
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

        for i in 1.. {
            let uniform_data = MainPassRawUniformData { step: stepsize };
            queue.write_buffer(
                &self.main_uniform_buffer,
                0,
                bytemuck::bytes_of(&uniform_data),
            );

            self.main_pass
                .render(&screenpass::ScreenPassRenderDescriptor {
                    device,
                    queue,
                    bind_group_resources: &[
                        self.main_uniform_buffer.as_entire_binding(),
                        wgpu::BindingResource::TextureView(
                            &self.temp_textures[1 - (i % 2)]
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    ],
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.temp_textures[i % 2]
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                        resolve_target: None,
                        ops: wgpu::Operations::default(),
                    })],
                });

            stepsize /= 2;
            // TODO: make a nonhacky way to end on temp_textures[0]
            if stepsize == 0 {
                if i % 2 == 1 {
                    stepsize = 1;
                } else {
                    break;
                }
            }
        }

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
