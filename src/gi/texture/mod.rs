use super::GIRenderer;
use crate::screenpass::{self, ScreenPass};
use egui_wgpu::wgpu;

pub struct TextureRenderer {
    pub label: String,

    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    screenpass: ScreenPass,
}

impl TextureRenderer {
    pub fn new(device: &wgpu::Device, texture_size: (u32, u32), label: String) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture renderer sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout_binding_types: &[_] = &[
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        ];
        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let screenpass = ScreenPass::new(
            device,
            Some("texture renderer"),
            bind_group_layout_binding_types,
            shader_module,
            &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        );

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture texture"),
            size: wgpu::Extent3d {
                width: texture_size.0,
                height: texture_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        TextureRenderer {
            label,
            sampler,
            screenpass,
            texture,
        }
    }

    fn load_texture_from_file(
        &mut self,
        filename: String,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let texture_image = match image::load(
            std::io::BufReader::new(match std::fs::File::open(filename) {
                Ok(f) => f,
                Err(e) => {
                    println!("Error opening file: {}", e);
                    return;
                }
            }),
            image::ImageFormat::Png,
        ) {
            Ok(img) => img,
            Err(e) => {
                println!("Error loading image: {}", e);
                return;
            }
        };

        let texture_rgba = texture_image.to_rgba8();
        let dimensions = texture_rgba.dimensions();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene texture"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        self.texture = texture;
    }
}

impl GIRenderer for TextureRenderer {
    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        let in_texture = &self.texture;
        self.screenpass
            .render(&screenpass::ScreenPassRenderDescriptor {
                device,
                queue,
                bind_group_resources: &[
                    wgpu::BindingResource::TextureView(
                        &in_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                    wgpu::BindingResource::Sampler(&self.sampler),
                ],
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ops: wgpu::Operations::default(),
                    resolve_target: None,
                })],
            });
    }

    fn render_egui(&mut self, ctx: &egui::Context, device: &wgpu::Device, queue: &wgpu::Queue) {
        egui::Window::new(&self.label)
            .default_size(egui::Vec2::new(1., 1.))
            .show(ctx, |ui| {
                if ui.button("Load Image").clicked() {
                    if let Some(filename) = native_dialog::FileDialog::new()
                        .show_open_single_file()
                        .unwrap()
                    {
                        self.load_texture_from_file(
                            filename.into_os_string().into_string().unwrap(),
                            device,
                            queue,
                        );
                    }
                }
            });
    }

    fn label(&self) -> String {
        self.label.clone()
    }
}
