use crate::screenpass::{self, ScreenPass};
use egui_wgpu::wgpu;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct RawUniformData {
    color: [f32; 3],
    radius: f32,
    texture_size: [f32; 2],
    // in pixels
    pos: [f32; 2],
}

pub struct Brush {
    screenpass: ScreenPass,
    uniform_buffer: wgpu::Buffer,
}

impl Brush {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Brush uniform buffer"),
            size: std::mem::size_of::<RawUniformData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout_binding_types = vec![vec![wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }]];
        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let screenpass = ScreenPass::new(device, bind_group_layout_binding_types, shader_module);

        Self {
            screenpass,
            uniform_buffer,
        }
    }

    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        out_texture: &wgpu::Texture,
        color: [f32; 3],
        pos: [f32; 2],
        radius: f32,
    ) {
        let uniform_data = RawUniformData {
            color,
            radius,
            texture_size: [out_texture.width() as f32, out_texture.height() as f32],
            pos,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

        self.screenpass
            .render(&screenpass::ScreenPassRenderDescriptor {
                device: &device,
                queue: &queue,
                out_texture,
                fragment_targets: &[Some(wgpu::ColorTargetState {
                    format: out_texture.format(),
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                bind_group_resources: &[&[self.uniform_buffer.as_entire_binding()]],
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
            })
    }
}
