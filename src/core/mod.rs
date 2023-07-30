use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
    window::WindowBuilder,
};

/// Create and display the main window
pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Application State holding the WGPU Surface
    let mut app_state = AppState::new(window).await;

    // Event loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == app_state.window().id() => {
            if !app_state.input(event) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,

                    // Resize
                    WindowEvent::Resized(physical_size) => {
                        app_state.resize(*physical_size);
                    }
                    // Moved between monitors with different DPIs?
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        // new_inner_size is &&mut so we have to dereference it twice
                        app_state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        // Redraw
        Event::RedrawRequested(window_id) if window_id == app_state.window().id() => {
            app_state.update();
            match app_state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => app_state.resize(app_state.size),
                // Exit if we are OOM
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => {
                    eprintln!("{e:?}");
                }
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            app_state.window().request_redraw();
        }
        _ => {}
    });
}

struct AppState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
}

impl AppState {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to the actual GPU
        // Choosing all backends means: Vulkan | Metal | DX12 | Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // The 'surface' represents the part of the window that we can
        // draw to.  It needs to live as long as the window that created it.
        // The 'AppState' owns the window, so while this is unsafe code,
        // it should practically be okay.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter_options = wgpu::RequestAdapterOptions {
            // HighPerformance will favour performance over battery life
            power_preference: wgpu::PowerPreference::HighPerformance,
            // This tells wgpu to find an adapter that can present
            // to the supplied surface
            compatible_surface: Some(&surface),
            // Forces wgpu to pick an adapter that will work on all hardware
            // This might mean that the rendering backend will be software instead
            // of hardware accelerated on the GPU
            force_fallback_adapter: false,
        };

        let adapter = instance.request_adapter(&adapter_options).await.unwrap();
        println!("Adapter: {adapter:?}");

        let device_description = wgpu::DeviceDescriptor {
            // This allows you to choose extra features you might want
            features: wgpu::Features::empty(),
            // More about limits: https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
            limits: wgpu::Limits::default(),
            label: None,
        };
        let trace_path = None;

        let (device, queue) = adapter
            .request_device(&device_description, trace_path)
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        eprintln!("Format supported by this surface:");
        for surface_format in surface_capabilities.formats.iter() {
            eprintln!("{:?}", surface_format);
        }

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        // Red, green, blue, and alpha channels. 16 bit float per channel. Float in shader.
        // let surface_format = wgpu::TextureFormat::Rgba16Float;
        eprintln!("Surface format chosen: {surface_format:?}");

        // TODO:
        // Make sure that the width and height of the `SurfaceTexture` are not 0,
        // as that can cause your app to crash.

        let surface_config = wgpu::SurfaceConfiguration {
            // 'RENDER_ATTACHMENTS' specifies that the texture will be used
            // to write to the screen
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            // present_mode: surface_capabilities.present_modes[0],
            // This caps the display rate at the displays framerate:
            // which is essentially VSync
            present_mode: wgpu::PresentMode::Fifo,
            // alpha_mode: surface_capabilities.alpha_modes[0],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            size,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Support the resizing of the window
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    // input() returns a bool to indicate whether an event has been fully processed.
    // If the method returns true, the main loop won't process the event any further.
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        // TODO:
    }

    /// Perform the actual magic of rendering to the window
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;

        // This line creates a TextureView with default settings.
        // We need to do this because we want to control how the render
        // code interacts with the texture.
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // We also need to create a CommandEncoder to create the actual
        // commands to send to the gpu. Most modern graphics frameworks
        // expect commands to be stored in a command buffer before being
        // sent to the gpu. The encoder builds a command buffer that we
        // can then send to the gpu.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as view unless multisampling is enabled.
                // We don't need to specify this, so we leave it as None.
                resolve_target: None,
                // These are the operations that should be performed by the GPU
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    // This tells wgpu to store the rendered result to the Texture
                    // behind our TextureView (in this case, the SurfaceTexture)
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        // begin_render_pass() borrows encoder mutably (aka &mut self).
        // We can't call encoder.finish() until we release that mutable borrow,
        // which we do manually via the explicit drop()
        drop(render_pass);

        // This tells wgpu to 'finish' the command buffer
        // and submit it to the GPU queue
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}
