mod raymarcher;

use egui_wgpu::wgpu;

trait GIRenderer {
    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    );

    #[allow(unused_variables)]
    fn render_egui(&mut self, ctx: &egui::Context) {}
}

// manages all of gi stuffs
pub struct GI {
    renderers: Vec<Box<dyn GIRenderer>>,
    cur_renderer: usize,
}

impl GI {
    pub fn new(device: &wgpu::Device, window_size: (u32, u32)) -> Self {
        let default_renderer =
            raymarcher::Raymarcher::new(&device, window_size, wgpu::TextureFormat::Rgba32Float);
        GI {
            renderers: vec![Box::new(default_renderer)],
            cur_renderer: 0,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        self.renderers[self.cur_renderer].render(device, queue, in_texture, out_texture);
    }
}
