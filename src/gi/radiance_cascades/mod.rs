mod config;
mod resources;

use super::GIRenderer;
use crate::jfa::JFA;
use egui_wgpu::wgpu;

use config::RCConfig;
use config::RawUniformData;
use resources::RCResources;

pub struct RadianceCascades {
    pub label: String,

    config: RCConfig,
    gui_config: RCConfig,
    window_size: (u32, u32),

    jfa: JFA,
    resources: RCResources,
}

impl RadianceCascades {
    pub fn new(device: &wgpu::Device, window_size: (u32, u32), label: String) -> Self {
        let config = RCConfig::default();

        let resources = RCResources::new(device, window_size, config);
        let jfa = JFA::new(device, window_size);

        RadianceCascades {
            label,

            config,
            gui_config: config,
            window_size,

            jfa,
            resources,
        }
    }
}

impl GIRenderer for RadianceCascades {
    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        in_texture: &wgpu::Texture,
        out_texture: &wgpu::Texture,
    ) {
        let in_view = in_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let out_view = out_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.jfa.render(
            device,
            queue,
            &in_view,
            &self.resources.sdf_view,
            (in_texture.size().width, in_texture.size().height),
        );

        let in_texture_bind_group = self.resources.create_texture_bind_group(device, &in_view);

        for i in 0..self.config.num_cascades {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // TODO: not do this
            let uniform_data = RawUniformData {
                cur_cascade: self.config.num_cascades - i - 1,
                ..RawUniformData::from(self.config)
            };
            queue.write_buffer(
                &self.resources.uniform_buffer,
                0,
                bytemuck::bytes_of(&uniform_data),
            );

            {
                let mut compute_pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                compute_pass.set_pipeline(&self.resources.main_pipeline);
                compute_pass.set_bind_group(0, &self.resources.uniform_bind_group, &[]);
                compute_pass.set_bind_group(1, &in_texture_bind_group, &[]);
                compute_pass.set_bind_group(
                    2,
                    &self.resources.temp_bind_groups[i as usize % 2],
                    &[],
                );

                let cascade_size = self
                    .config
                    .get_cascade_size(self.window_size, self.config.num_cascades - i - 1);
                let num_groups = u32::div_ceil(cascade_size, 128);

                compute_pass.dispatch_workgroups(num_groups, 1, 1);
            }
            queue.submit(Some(encoder.finish()));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let final_bind_group = self.resources.create_final_bind_group(
            device,
            &out_view,
            1 - (self.config.num_cascades % 2) as usize,
        );

        {
            let mut final_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            final_pass.set_pipeline(&self.resources.final_pipeline);
            final_pass.set_bind_group(0, &final_bind_group, &[]);
            final_pass.dispatch_workgroups(
                u32::div_ceil(out_texture.width(), 16),
                u32::div_ceil(out_texture.height(), 16),
                1,
            );
        }

        queue.submit(Some(encoder.finish()));
    }

    fn render_egui(&mut self, ctx: &egui::Context, device: &wgpu::Device, _: &wgpu::Queue) {
        let max_cascade_size = {
            let max_buffer_elems =
                device.limits().max_buffer_size / RCResources::CASCADE_BUFFER_ELEM_SIZE as u64;
            let max_workgroups = device.limits().max_compute_workgroups_per_dimension;
            u32::min(max_buffer_elems as u32, max_workgroups * 128)
        };

        egui::Window::new(&self.label)
            .default_size(egui::Vec2::new(1., 1.))
            .show(ctx, |ui| {
                let size_label_color =
                    if self.gui_config.get_max_cascade_size(self.window_size) > max_cascade_size {
                        egui::Color32::from_rgb(255, 0, 0)
                    } else {
                        egui::Color32::from_rgb(200, 200, 200)
                    };
                ui.colored_label(
                    size_label_color,
                    format!(
                        "Max cascade size: {}",
                        self.gui_config.get_max_cascade_size(self.window_size)
                    ),
                );

                ui.heading("C0 raylength");
                ui.add(
                    egui::Slider::new(&mut self.gui_config.c0_raylength, 0.5..=512.)
                        .logarithmic(true),
                );

                ui.heading("C0 probe spacing");
                ui.add(
                    egui::Slider::new(&mut self.gui_config.c0_spacing, 0.25..=16.).step_by(0.25),
                );

                ui.heading("C0 ray count");
                ui.add(egui::Slider::new(&mut self.gui_config.c0_rays, 3..=256).logarithmic(true));

                ui.heading("Spatial scaling");
                ui.label(format!(
                    "per axis, total is {}",
                    self.gui_config.spatial_scaling.powi(2)
                ));
                ui.add(egui::Slider::new(
                    &mut self.gui_config.spatial_scaling,
                    1.1..=9.0,
                ));

                ui.heading("Angular scaling");
                ui.add(egui::Slider::new(
                    &mut self.gui_config.angular_scaling,
                    2..=16,
                ));

                ui.heading("Cascade count");
                ui.add(egui::Slider::new(&mut self.gui_config.num_cascades, 1..=16));

                ui.heading("Probe layout");
                ui.columns(2, |columns| {
                    columns[0].radio_value(
                        &mut self.gui_config.probe_layout,
                        config::ProbeLayout::Offset,
                        "Offset",
                    );
                    columns[1].radio_value(
                        &mut self.gui_config.probe_layout,
                        config::ProbeLayout::Stacked,
                        "Stacked",
                    );
                });

                egui::ComboBox::from_label("Ringing Fix")
                    .selected_text(format!("{}", self.gui_config.ringing_fix))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.gui_config.ringing_fix,
                            config::RingingFix::Vanilla,
                            "Vanilla",
                        );

                        ui.selectable_value(
                            &mut self.gui_config.ringing_fix,
                            config::RingingFix::Bilinear,
                            "Bilinear",
                        );
                    });
            });

        if self.gui_config.get_max_cascade_size(self.window_size) > max_cascade_size {
            println!("Config ignored, the cascades are too big");
            return;
        }

        if self.config != self.gui_config {
            self.config = self.gui_config;
            self.resources = RCResources::new(device, self.window_size, self.config);
        }
    }

    fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        self.window_size = new_size;
        self.resources = RCResources::new(device, new_size, self.config);
        self.jfa = JFA::new(device, new_size);
    }

    fn label(&self) -> String {
        self.label.clone()
    }
}
