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
                compute_pass.dispatch_workgroups(
                    u32::div_ceil(self.resources.temp_textures[0].width(), 16),
                    u32::div_ceil(self.resources.temp_textures[0].height(), 16),
                    1,
                );
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

    fn render_egui(&mut self, ctx: &egui::Context, device: &wgpu::Device) {
        let config_before_egui = self.config;

        egui::Window::new(&self.label).show(ctx, |ui| {
            ui.heading("rc !!!");

            ui.heading("c0 raylength");
            ui.add(egui::Slider::new(&mut self.config.c0_raylength, 0.5..=512.).logarithmic(true));

            ui.heading("c0 probe spacing");
            ui.add(egui::Slider::new(&mut self.config.c0_spacing, 0.25..=16.).step_by(0.25));

            ui.heading("c0 ray count");
            ui.add(egui::Slider::new(&mut self.config.c0_rays, 3..=256).logarithmic(true));

            ui.heading("spatial scaling");
            ui.label(format!(
                "per axis, total is {}",
                self.config.spatial_scaling.powi(2)
            ));
            ui.add(egui::Slider::new(
                &mut self.config.spatial_scaling,
                1.1..=4.0,
            ));

            ui.heading("angular scaling");
            ui.add(egui::Slider::new(&mut self.config.angular_scaling, 2..=16));

            ui.heading("cascade number");
            ui.add(egui::Slider::new(&mut self.config.num_cascades, 1..=16));
        });

        if self.config != config_before_egui {
            self.resources = RCResources::new(device, self.window_size, self.config);
        }
    }

    fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        self.window_size = new_size;
        self.resources = RCResources::new(device, new_size, self.config);
        self.jfa = JFA::new(device, new_size);
    }
}
