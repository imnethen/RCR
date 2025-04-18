mod difference;
mod radiance_cascades;
mod raymarcher;
mod texture;

use difference::Difference;
use egui_wgpu::wgpu;
use radiance_cascades::RadianceCascades;
use raymarcher::Raymarcher;
use texture::TextureRenderer;

trait GIRenderer {
    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    );

    #[allow(unused_variables)]
    fn render_egui(&mut self, ctx: &egui::Context, device: &wgpu::Device, queue: &wgpu::Queue) {}

    #[allow(unused_variables)]
    fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {}

    fn label(&self) -> String {
        "NO LABEL".to_string()
    }
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
        let default_renderer = RadianceCascades::new(&device, window_size, "RC 0".to_owned());
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
        {
            let old_config = self.difference.config;
            self.difference = Difference::new(device, new_size);
            self.difference.config = old_config;
        }
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

    pub fn render_egui(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, ctx: &egui::Context) {
        egui::Window::new("Renderers")
            .default_size(egui::Vec2::new(180., 1.))
            .show(ctx, |ui| {
                if ui.button("New Raymarcher").clicked() {
                    self.renderers.push(Box::new(Raymarcher::new(
                        device,
                        self.cur_window_size,
                        wgpu::TextureFormat::Rgba16Float,
                        format!("Raymarcher {}", self.renderers.len()),
                    )));
                }
                if ui.button("New Radiance Cascades").clicked() {
                    self.renderers.push(Box::new(RadianceCascades::new(
                        device,
                        self.cur_window_size,
                        format!("RC {}", self.renderers.len()),
                    )));
                }
                if ui.button("New Texture Renderer").clicked() {
                    self.renderers.push(Box::new(TextureRenderer::new(
                        device,
                        self.cur_window_size,
                        format!("Texture {}", self.renderers.len()),
                    )));
                }
                ui.separator();
                ui.heading("Current");
                ui.radio_value(&mut self.cur_renderer, CurRenderer::Diff, "Difference");
                for i in 0..self.renderers.len() {
                    ui.radio_value(
                        &mut self.cur_renderer,
                        CurRenderer::Index(i),
                        self.renderers[i].label(),
                    );
                }
            });

        match self.cur_renderer {
            CurRenderer::Diff => {
                egui::Window::new("Difference")
                    .default_size(egui::Vec2::new(1., 1.))
                    .show(ctx, |ui| {
                        ui.heading("Choose renderers");
                        egui::ComboBox::from_label("First")
                            .selected_text(self.renderers[self.diff_indices.0].label())
                            .show_ui(ui, |ui| {
                                self.renderers.iter().enumerate().for_each(|(i, r)| {
                                    ui.selectable_value(&mut self.diff_indices.0, i, r.label());
                                });
                            });
                        egui::ComboBox::from_label("Second")
                            .selected_text(self.renderers[self.diff_indices.1].label())
                            .show_ui(ui, |ui| {
                                self.renderers.iter().enumerate().for_each(|(i, r)| {
                                    ui.selectable_value(&mut self.diff_indices.1, i, r.label());
                                });
                            });

                        ui.heading("Multiplier");
                        ui.add(
                            egui::Slider::new(&mut self.difference.config.mult, 1.0..=200.)
                                .logarithmic(true),
                        );

                        ui.heading("Difference mode");
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::Abs,
                            "Abs",
                        );
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::FirstMinusSecond,
                            "First - second",
                        );
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::SecondMinusFirst,
                            "Second - first",
                        );
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::First,
                            "First",
                        );
                        ui.radio_value(
                            &mut self.difference.config.mode,
                            difference::DiffMode::Second,
                            "Second",
                        );
                    });

                if self.diff_indices.0 < self.renderers.len() {
                    self.renderers[self.diff_indices.0].render_egui(ctx, device, queue);
                }
                if self.diff_indices.1 < self.renderers.len() {
                    self.renderers[self.diff_indices.1].render_egui(ctx, device, queue);
                }
            }
            CurRenderer::Index(i) => self.renderers[i].render_egui(ctx, device, queue),
        };
    }
}
