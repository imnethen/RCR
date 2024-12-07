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
        let jfa = JFA::new(device, window_size, RCResources::SDF_FORMAT);

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
        let uniform_data = RawUniformData::from(self.config);
        queue.write_buffer(
            &self.resources.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniform_data),
        );

        let in_view = in_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let out_view = out_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.jfa
            .render(device, queue, in_texture, &self.resources.sdf_texture);

        let in_texture_bind_group = self.resources.create_texture_bind_group(device, &in_view);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.resources.main_pipeline);
            compute_pass.set_bind_group(0, &self.resources.uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, &in_texture_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.resources.temp_bind_groups[0], &[]);
            compute_pass.dispatch_workgroups(
                (self.window_size.0 + 15) / 16,
                (self.window_size.1 + 15) / 16,
                1,
            );
        }

        let final_bind_group = self.resources.create_final_bind_group(device, &out_view, 0);

        {
            let mut final_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            final_pass.set_pipeline(&self.resources.final_pipeline);
            final_pass.set_bind_group(0, &final_bind_group, &[]);
            final_pass.dispatch_workgroups(
                (out_texture.width() + 15) / 16,
                (out_texture.height() + 15) / 16,
                1,
            );
        }

        queue.submit(Some(encoder.finish()));
    }

    fn render_egui(&mut self, ctx: &egui::Context) {
        //egui::Window::new(&self.label).show(ctx, |ui| {
        //ui.add(egui::Slider::new(&mut self.config.ray_count, 4..=8196).logarithmic(true));
        //});
    }

    fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        self.window_size = new_size;
        self.resources = RCResources::new(device, new_size, self.config);
        self.jfa = JFA::new(device, new_size, RCResources::SDF_FORMAT);
    }
}
