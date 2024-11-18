mod brush;
mod gi;
mod inpututil;
mod jfa;
mod jfa2;
mod screenpass;
mod texturerenderer;

use brush::Brush;
use gi::GI;
use jfa2::JFA;
use texturerenderer::TextureRenderer;

use egui_wgpu::wgpu;
use winit::event::WindowEvent;

use inpututil::InputController;

struct State<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,

    window: &'a winit::window::Window,
    surface: wgpu::Surface<'a>,
    config: wgpu::SurfaceConfiguration,

    input_controller: InputController,

    brush: Brush,
    texture_renderer: TextureRenderer,
    gi: GI,
    temp_jfa: JFA,

    in_texture: wgpu::Texture,
}

impl<'a> State<'a> {
    async fn new(window: &'a winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::PUSH_CONSTANTS,
                    required_limits: wgpu::Limits {
                        max_push_constant_size: 4,
                        ..Default::default()
                    },
                },
                None,
            )
            .await
            .unwrap();

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        let brush = Brush::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let texture_renderer =
            TextureRenderer::new(&device, wgpu::FilterMode::Linear, config.format);
        let gi = GI::new(&device);

        let input_controller = InputController::default();

        let in_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("in texture"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let temp_jfa = JFA::new(&device, (size.width, size.height), config.format);

        State {
            device,
            queue,

            window,
            surface,
            config,

            brush,
            texture_renderer,
            gi,

            temp_jfa,

            input_controller,

            in_texture,
        }
    }

    fn render(&mut self) {
        let output = self.surface.get_current_texture().unwrap();

        if self
            .input_controller
            .mouse_button_pressed(winit::event::MouseButton::Left)
        {
            self.brush.draw(
                &self.device,
                &self.queue,
                &self.in_texture,
                [1., 1., 1.],
                self.input_controller.get_mouse_pos().into(),
                30.,
            );
        }

        self.temp_jfa
            .render(&self.device, &self.queue, &self.in_texture, &output.texture);

        // self.texture_renderer
        //     .render(&self.device, &self.queue, &self.in_texture, &output.texture);

        // self.gi
        //     .render(&self.device, &self.queue, &self.in_texture, &output.texture);

        output.present();
    }
}

pub async fn run() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .with_inner_size(winit::dpi::PhysicalSize::new(2048, 1024))
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&window).await;

    event_loop
        .run(move |event, target| match event {
            winit::event::Event::WindowEvent {
                ref event,
                window_id: _,
            } => {
                let consumed = state.input_controller.process_event(&event);
                // if consumed {
                //     return;
                // }

                match event {
                    WindowEvent::Resized(new_size) => {
                        // TODO: resize in_texture and all the other stuff
                        state.config.width = new_size.width;
                        state.config.height = new_size.height;
                        state.surface.configure(&state.device, &state.config);
                    }
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::RedrawRequested => {
                        let start = std::time::Instant::now();
                        state.render();
                        println!("{:?}", std::time::Instant::now() - start);
                        state.input_controller.init_frame();
                        state.window.request_redraw();
                    }
                    WindowEvent::MouseInput {
                        device_id: _,
                        state: _,
                        button: _,
                    } => state.window.request_redraw(),
                    _ => {}
                }
            }
            _ => {}
        })
        .unwrap();
}
