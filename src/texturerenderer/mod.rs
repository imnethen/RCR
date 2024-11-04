use crate::screenpass::{self, ScreenPass};
use egui_wgpu::wgpu;

/// struct that copies a texture from one to another
///
/// for situations where encoder.copy_texture_to_texture
/// doesnt work, like when rendering to surface
///
/// only copies float textures
pub struct TextureRenderer {
    sampler: wgpu::Sampler,
    screenpass: ScreenPass,
}

impl TextureRenderer {
    pub fn new(device: &wgpu::Device, filter_mode: wgpu::FilterMode) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture renderer sampler"),
            mag_filter: filter_mode,
            min_filter: filter_mode,
            ..Default::default()
        });

        let bind_group_layout_binding_types: &[&[_]] = &[&[
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        ]];
        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let screenpass = ScreenPass::new(device, bind_group_layout_binding_types, shader_module);

        TextureRenderer {
            sampler,
            screenpass,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        self.screenpass
            .render(&screenpass::ScreenPassRenderDescriptor {
                device,
                queue,
                fragment_targets: &[Some(wgpu::ColorTargetState {
                    format: out_texture.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                bind_group_resources: &[&[
                    wgpu::BindingResource::TextureView(
                        &in_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                    wgpu::BindingResource::Sampler(&self.sampler),
                ]],
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ops: wgpu::Operations::default(),
                    resolve_target: None,
                })],
            });
    }
}
