use crate::brush::Brush;
use crate::InputController;
use egui_wgpu::wgpu;

struct SceneConfig {
    brush_color_left: [f32; 3],
    brush_color_right: [f32; 3],
    brush_radius: f32,
}

impl Default for SceneConfig {
    fn default() -> Self {
        SceneConfig {
            brush_color_left: [1., 1., 1.],
            brush_color_right: [0., 0., 0.],
            brush_radius: 30.,
        }
    }
}

pub struct Scene {
    config: SceneConfig,
    brush: Brush,
    texture: wgpu::Texture,
}

impl Scene {
    pub fn new(device: &wgpu::Device, texture_size: (u32, u32)) -> Self {
        let config = SceneConfig::default();
        let brush = Brush::new(device, wgpu::TextureFormat::Rgba8Unorm);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Scene {
            config,
            brush,
            texture,
        }
    }

    fn clear_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !device.features().contains(wgpu::Features::CLEAR_TEXTURE) {
            // TODO
            println!("oops ! no clear texture feature");
        }
        let mut clear_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        clear_encoder.clear_texture(&self.texture, &wgpu::ImageSubresourceRange::default());
        queue.submit(Some(clear_encoder.finish()));
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input_controller: &InputController,
    ) {
        if input_controller.key_just_pressed(winit::keyboard::KeyCode::Space) {
            self.clear_texture(device, queue);
        }

        if input_controller.mouse_button_pressed(winit::event::MouseButton::Left) {
            self.brush.draw(
                device,
                queue,
                &self.texture,
                self.config.brush_color_left,
                input_controller.get_mouse_pos().into(),
                self.config.brush_radius,
            );
        }

        if input_controller.mouse_button_pressed(winit::event::MouseButton::Right) {
            self.brush.draw(
                device,
                queue,
                &self.texture,
                self.config.brush_color_right,
                input_controller.get_mouse_pos().into(),
                self.config.brush_radius,
            );
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn render_egui(&mut self, ctx: &egui::Context) {
        egui::Window::new("scene").show(ctx, |ui| {
            ui.heading("brush radius");
            let brush_radius_slider = egui::Slider::new(&mut self.config.brush_radius, 1.0..=1024.)
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
