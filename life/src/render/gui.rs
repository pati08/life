use egui::TexturesDelta;
use std::time::Instant;
use std::{iter, sync::Arc};
use wgpu::Surface;

use ::egui::FontDefinitions;
use chrono::Timelike;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::Device;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event},
};

pub struct GuiState<'a> {
    platform: Platform,
    render_pass: RenderPass,
    app: egui_demo_lib::DemoWindows,
    device: Arc<Device>,
    surface: Arc<Surface<'a>>,
    start_time: Instant,
    window: Arc<winit::window::Window>,
}

impl<'a> GuiState<'a> {
    pub fn handle_event<T>(&mut self, event: &Event<T>) -> bool {
        let is_keyup = matches!(
            event,
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::KeyboardInput {
                    event: winit::event::KeyEvent {
                        state: ElementState::Released,
                        ..
                    },
                    ..
                },
                ..
            }
        );
        let captures = self.platform.captures_event(event);
        if !is_keyup {
            self.platform.handle_event(event);
        } else {
            return false;
        }
        captures
    }

    pub fn new(
        size: PhysicalSize<u32>,
        window: Arc<winit::window::Window>,
        device: Arc<wgpu::Device>,
        surface_format: wgpu::TextureFormat,
        surface: Arc<Surface<'a>>,
    ) -> GuiState<'a> {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let render_pass = RenderPass::new(&device, surface_format, 1);
        let app = egui_demo_lib::DemoWindows::default();
        Self {
            platform,
            render_pass,
            app,
            device,
            start_time: Instant::now(),
            surface,
            window,
        }
    }

    pub fn render(
        &mut self,
        surface_config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        mut encoder: wgpu::CommandEncoder,
    ) -> (wgpu::CommandEncoder, TexturesDelta) {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());

        // Begin to draw the UI frame.
        self.platform.begin_frame();

        // Draw the demo application.
        self.app.ui(&self.platform.context());
        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let full_output = self.platform.end_frame(Some(&self.window));
        let paint_jobs = self
            .platform
            .context()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: surface_config.width,
            physical_height: surface_config.height,
            scale_factor: self.window.scale_factor() as f32,
        };
        let tdelta: egui::TexturesDelta = full_output.textures_delta;
        self.render_pass
            .add_textures(&self.device, queue, &tdelta)
            .expect("add texture ok");
        self.render_pass
            .update_buffers(&self.device, queue, &paint_jobs, &screen_descriptor);

        // Record all render passes.
        self.render_pass
            .execute(
                &mut encoder,
                view,
                &paint_jobs,
                &screen_descriptor,
                // Some(wgpu::Color::BLACK),
                None,
            )
            .unwrap();
        (encoder, tdelta)
    }

    pub fn remove_textures(&mut self, tdelta: TexturesDelta) {
        self.render_pass
            .remove_textures(tdelta)
            .expect("remove texture ok");
    }
}
