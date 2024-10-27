mod brush;
mod inpututil;
mod texturerenderer;

use brush::Brush;

use egui_wgpu::wgpu;
use winit::event::WindowEvent;

use inpututil::InputController;

struct State<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,

    window: &'a winit::window::Window,
    surface: wgpu::Surface<'a>,
    config: wgpu::SurfaceConfiguration,

    brush: Brush,

    input_controller: InputController,
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
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        config.format = wgpu::TextureFormat::Rgba8Unorm;
        surface.configure(&device, &config);

        let brush = Brush::new(&device);

        let input_controller = InputController::default();

        State {
            device,
            queue,
            window,
            surface,
            config,
            brush,
            input_controller,
        }
    }

    fn render(&mut self) {
        let output = self.surface.get_current_texture().unwrap();
        // let output_view = output
        //     .texture
        //     .create_view(&wgpu::TextureViewDescriptor::default());

        self.brush.draw(
            &self.device,
            &self.queue,
            &output.texture,
            [1., 1., 1.],
            self.input_controller.get_mouse_pos().into(),
            100.,
        );

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
                if consumed {
                    return;
                }

                match event {
                    WindowEvent::Resized(new_size) => {
                        state.config.width = new_size.width;
                        state.config.height = new_size.height;
                        state.surface.configure(&state.device, &state.config);
                    }
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::RedrawRequested => {
                        state.render();
                        state.input_controller.init_frame();
                        state.window.request_redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .unwrap();
}
