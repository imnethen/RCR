mod brush_circle;
mod brush_square;
mod egui_renderer;
mod gi;
mod inpututil;
mod jfa;
mod scene;
mod screenpass;
mod texturerenderer;

use egui_renderer::EguiRenderer;
use gi::GI;
use scene::Scene;
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

    texture_renderer: TextureRenderer,
    gi: GI,
    egui_renderer: EguiRenderer,

    scene: Scene,
    out_texture: wgpu::Texture,
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
                    required_features: wgpu::Features::PUSH_CONSTANTS
                        | wgpu::Features::FLOAT32_FILTERABLE
                        | wgpu::Features::CLEAR_TEXTURE,
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

        let texture_renderer =
            TextureRenderer::new(&device, wgpu::FilterMode::Linear, config.format);
        let gi = GI::new(&device, (size.width, size.height));
        let egui_renderer = EguiRenderer::new(&device, config.format, &window);

        let input_controller = InputController::default();

        let scene = Scene::new(&device, (size.width, size.height));

        let out_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("out texture"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        State {
            device,
            queue,

            window,
            surface,
            config,

            texture_renderer,
            gi,
            egui_renderer,

            input_controller,

            scene,
            out_texture,
        }
    }

    fn render_egui(&mut self, out_texture_view: &wgpu::TextureView) {
        self.egui_renderer.render(
            &self.device,
            &self.queue,
            &self.window,
            out_texture_view,
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: self.window.inner_size().into(),
                pixels_per_point: 1.,
            },
            |ctx| {
                ctx.style_mut(|style| style.visuals.window_shadow = egui::epaint::Shadow::NONE);

                self.scene.render_egui(ctx);
                self.gi.render_egui(&self.device, ctx);
            },
        );
    }

    fn render(&mut self) {
        let output = self.surface.get_current_texture().unwrap();

        self.scene
            .update(&self.device, &self.queue, &self.input_controller);

        self.gi.render(
            &self.device,
            &self.queue,
            &self.scene.texture(),
            &self.out_texture,
        );

        self.texture_renderer.render(
            &self.device,
            &self.queue,
            &self.out_texture,
            &output.texture,
        );

        self.render_egui(&output.texture.create_view(&Default::default()));

        output.present();
    }
}

pub async fn run() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080))
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&window).await;

    event_loop
        .run(move |event, target| match event {
            winit::event::Event::WindowEvent {
                ref event,
                window_id: _,
            } => {
                let consumed_by_egui = state
                    .egui_renderer
                    .handle_input(&state.window, event)
                    .consumed;
                if consumed_by_egui {
                    return;
                }

                let _consumed_by_ic = state.input_controller.process_event(&event);

                match event {
                    WindowEvent::Resized(new_size) => {
                        // TODO: resize in_texture and all the other stuff and egui and ashnteoioi
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
