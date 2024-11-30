mod difference;
mod radiance_cascades;
mod raymarcher;

use difference::Difference;
use egui_wgpu::wgpu;
use radiance_cascades::RadianceCascades;
use raymarcher::Raymarcher;

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

    #[allow(unused_variables)]
    fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {}
}

#[derive(PartialEq)]
enum CurRenderer {
    Diff,
    Index(usize),
}

// manages all of gi stuffs
pub struct GI {
    renderers: Vec<Box<dyn GIRenderer>>,
    cur_renderer: CurRenderer,

    difference: Difference,
    diff_indices: (usize, usize),

    cur_window_size: (u32, u32),
}

impl GI {
    pub fn new(device: &wgpu::Device, window_size: (u32, u32)) -> Self {
        let default_renderer = RadianceCascades::new(
            &device,
            window_size,
            wgpu::TextureFormat::Rgba32Float,
            "rc 0".to_owned(),
        );
        GI {
            renderers: vec![Box::new(default_renderer)],
            cur_renderer: CurRenderer::Index(0),

            difference: Difference::new(device, window_size),
            diff_indices: (0, 0),

            cur_window_size: window_size,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        self.cur_window_size = new_size;
        self.difference.resize(device, new_size);
        for i in 0..self.renderers.len() {
            self.renderers[i].resize(device, new_size);
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        match self.cur_renderer {
            CurRenderer::Diff => {
                if self.diff_indices.0 >= self.renderers.len()
                    || self.diff_indices.1 >= self.renderers.len()
                {
                    println!("invalid diff indices");
                    return;
                }

                let diff_textures = self.difference.textures();

                self.renderers[self.diff_indices.0].render(
                    device,
                    queue,
                    in_texture,
                    &diff_textures[0],
                );
                self.renderers[self.diff_indices.1].render(
                    device,
                    queue,
                    in_texture,
                    &diff_textures[1],
                );
                self.difference.render(device, queue, out_texture);
            }
            CurRenderer::Index(i) => {
                self.renderers[i].render(device, queue, in_texture, out_texture);
            }
        }
    }

    pub fn render_egui(&mut self, device: &wgpu::Device, ctx: &egui::Context) {
        egui::Window::new("bwaah").show(ctx, |ui| {
            if ui.button("new raymarcher").clicked() {
                self.renderers.push(Box::new(Raymarcher::new(
                    device,
                    self.cur_window_size,
                    wgpu::TextureFormat::Rgba32Float,
                    format!("raymarcher {}", self.renderers.len()),
                )));
            }
            if ui.button("new radiance_cascades").clicked() {
                self.renderers.push(Box::new(RadianceCascades::new(
                    device,
                    self.cur_window_size,
                    wgpu::TextureFormat::Rgba32Float,
                    format!("rc {}", self.renderers.len()),
                )));
            }
            ui.separator();
            ui.heading("current");
            ui.radio_value(&mut self.cur_renderer, CurRenderer::Diff, "Difference");
            for i in 0..self.renderers.len() {
                ui.radio_value(
                    &mut self.cur_renderer,
                    CurRenderer::Index(i),
                    format!("renderer {}", i),
                );
            }
        });

        match self.cur_renderer {
            CurRenderer::Diff => {
                egui::Window::new("diff")
                    .default_size(egui::Vec2::new(1., 1.))
                    .show(ctx, |ui| {
                        ui.heading("renderer indices");

                        ui.columns(2, |columns| {
                            columns[0].add(egui::DragValue::new(&mut self.diff_indices.0));
                            columns[1].add(egui::DragValue::new(&mut self.diff_indices.1));
                        });

                        ui.heading("multiplier");
                        ui.add(
                            egui::Slider::new(&mut self.difference.config.mult, 1.0..=200.)
                                .logarithmic(true),
                        );

                        ui.heading("difference mode");
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::Abs,
                            "abs",
                        );
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::FirstMinusSecond,
                            "first - second",
                        );
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::SecondMinusFirst,
                            "second - first",
                        );
                    });

                if self.diff_indices.0 < self.renderers.len() {
                    self.renderers[self.diff_indices.0].render_egui(ctx);
                }
                if self.diff_indices.1 < self.renderers.len() {
                    self.renderers[self.diff_indices.1].render_egui(ctx);
                }
            }
            CurRenderer::Index(i) => self.renderers[i].render_egui(ctx),
        };
    }
}
