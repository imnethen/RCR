use egui_wgpu::wgpu;

pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        out_texture_format: wgpu::TextureFormat,
        window: &winit::window::Window,
    ) -> Self {
        let context = egui::Context::default();

        // TODO: none none
        let state = egui_winit::State::new(context, egui::ViewportId::ROOT, window, None, None);
        let renderer = egui_wgpu::Renderer::new(device, out_texture_format, None, 1);

        EguiRenderer { state, renderer }
    }

    pub fn handle_input(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        window: &winit::window::Window,
        out_texture_view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        run_ui: impl FnOnce(&egui::Context),
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui renderer encoder"),
        });

        self.state
            .egui_ctx()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        let raw_input = self.state.take_egui_input(window);
        let full_output = self.state.egui_ctx().run(raw_input, |_| {
            run_ui(self.state.egui_ctx());
        });

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, &mut encoder, &tris, &screen_descriptor);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui renderer render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: out_texture_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                })],
                ..Default::default()
            });
            self.renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
        }

        for tex in &full_output.textures_delta.free {
            self.renderer.free_texture(tex);
        }
    }
}
