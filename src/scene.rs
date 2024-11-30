use crate::brush_square::Brush;
use crate::InputController;
use egui_wgpu::wgpu;

struct SceneConfig {
    brush_color_left: [f32; 3],
    brush_color_right: [f32; 3],
    brush_size: u32,
}

impl Default for SceneConfig {
    fn default() -> Self {
        SceneConfig {
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
                self.config.brush_color_left,
                mouse_pos,
                self.config.brush_size,
            );
        }

        if input_controller.mouse_button_pressed(winit::event::MouseButton::Right) {
            self.brush.draw(
                device,
                queue,
                &self.texture_view,
                self.config.brush_color_right,
                mouse_pos,
                self.config.brush_size,
            );
        }
    }

    pub fn render_egui(&mut self, ctx: &egui::Context) {
        egui::Window::new("scene").show(ctx, |ui| {
            ui.heading("brush radius");
            let brush_radius_slider = egui::Slider::new(&mut self.config.brush_size, 1..=1024)
                .logarithmic(true)
                .suffix("px");
            ui.add(brush_radius_slider);

            ui.heading("brush lmb color");
            ui.color_edit_button_rgb(&mut self.config.brush_color_left);

            ui.heading("brush rmb color");
            ui.color_edit_button_rgb(&mut self.config.brush_color_right);
        });
    }
}
