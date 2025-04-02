use crate::brush::Brush;
use crate::brush::BrushShape;
use crate::InputController;
use egui_wgpu::wgpu;

struct SceneConfig {
    brush_shape: BrushShape,
    brush_color_left: [f32; 3],
    brush_color_right: [f32; 3],
    brush_size: u32,
}

impl Default for SceneConfig {
    fn default() -> Self {
        SceneConfig {
            brush_shape: BrushShape::Circle,
            brush_color_left: [1., 1., 1.],
            brush_color_right: [0., 0., 0.],
            brush_size: 30,
        }
    }
}

pub struct Scene {
    config: SceneConfig,
    brush: Brush,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
}

impl Scene {
    fn create_texture(device: &wgpu::Device, texture_size: (u32, u32)) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene texture"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: texture_size.0,
                height: texture_size.1,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        })
    }

    pub fn new(device: &wgpu::Device, texture_size: (u32, u32)) -> Self {
        let config = SceneConfig::default();
        let brush = Brush::new(device);
        let texture = Scene::create_texture(device, texture_size);

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Scene {
            config,
            brush,
            texture,
            texture_view,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, texture_size: (u32, u32)) {
        self.texture = Scene::create_texture(device, texture_size);
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    fn clear_texture(&mut self, device: &wgpu::Device) {
        self.texture = Scene::create_texture(device, (self.texture.width(), self.texture.height()));
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
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

        self.texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.texture = texture;
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input_controller: &InputController,
    ) {
        if input_controller.key_just_pressed(winit::keyboard::KeyCode::Space) {
            self.clear_texture(device);
        }

        let mouse_pos = {
            let mpf = input_controller.get_mouse_pos();
            [mpf.0 as u32, mpf.1 as u32]
        };

        if input_controller.mouse_button_pressed(winit::event::MouseButton::Left) {
            self.brush.draw(
                device,
                queue,
                &self.texture_view,
                self.config.brush_shape,
                mouse_pos,
                self.config.brush_size,
                self.config.brush_color_left,
            );
        }

        if input_controller.mouse_button_pressed(winit::event::MouseButton::Right) {
            self.brush.draw(
                device,
                queue,
                &self.texture_view,
                self.config.brush_shape,
                mouse_pos,
                self.config.brush_size,
                self.config.brush_color_right,
            );
        }
    }

    pub fn render_egui(&mut self, ctx: &egui::Context, device: &wgpu::Device, queue: &wgpu::Queue) {
        egui::Window::new("scene")
            .default_size(egui::Vec2::new(1., 1.))
            .show(ctx, |ui| {
                ui.heading("brush shape");
                ui.columns(2, |columns| {
                    columns[0].radio_value(
                        &mut self.config.brush_shape,
                        BrushShape::Square,
                        "Square",
                    );
                    columns[1].radio_value(
                        &mut self.config.brush_shape,
                        BrushShape::Circle,
                        "Circle",
                    );
                });

                ui.heading("brush size");
                let brush_size_slider = egui::Slider::new(&mut self.config.brush_size, 1..=1024)
                    .logarithmic(true)
                    .suffix("px");
                ui.add(brush_size_slider);

                ui.heading("brush lmb color");
                ui.color_edit_button_rgb(&mut self.config.brush_color_left);

                ui.heading("brush rmb color");
                ui.color_edit_button_rgb(&mut self.config.brush_color_right);

                if ui.button("Clear Texture").clicked() {
                    self.clear_texture(&device);
                }

                if ui.button("Load Scene From File").clicked() {
                    if let Some(filename) = tinyfiledialogs::open_file_dialog("Open", "", None) {
                        self.load_texture_from_file(filename, device, queue);
                    }
                }
            });
    }
}
