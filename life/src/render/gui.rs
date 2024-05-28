use egui::{Context, Id, Slider, TexturesDelta};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use ::egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::Device;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event},
};

use crate::game::GameState;

pub struct GuiState {
    platform: Platform,
    render_pass: RenderPass,
    app: Gui,
    device: Arc<Device>,
    start_time: Instant,
    window: Arc<winit::window::Window>,
}

impl GuiState {
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
        game_state: Arc<Mutex<GameState>>,
    ) -> GuiState {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let render_pass = RenderPass::new(&device, surface_format, 1);
        let app = Gui { game_state };
        Self {
            platform,
            render_pass,
            app,
            device,
            start_time: Instant::now(),
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

struct Gui {
    game_state: Arc<Mutex<GameState>>,
}

impl Gui {
    const PLAYING_TEXT: &'static str = "Playing \u{23F8}";
    const NOT_PLAYING_TEXT: &'static str = "Stopped \u{23F5}";
    fn ui(&mut self, ctx: &Context) {
        let mut game = self.game_state.lock().unwrap();
        egui::containers::panel::TopBottomPanel::top(Id::new("top_panel")).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let button_text = if game.is_playing() {
                    Self::PLAYING_TEXT
                } else {
                    Self::NOT_PLAYING_TEXT
                };
                let play_button = ui.button(button_text);
                if play_button.clicked() {
                    game.toggle_playing(None);
                }
                let speed_get_set = |set: Option<f64>| {
                    if let Some(v) = set {
                        game.set_interval(std::time::Duration::from_secs_f64(v));
                    }
                    game.get_interval().as_secs_f64()
                };
                ui.label("Speed: ");
                let speed_slider = Slider::from_get_set(1f64..=0.01f64, speed_get_set)
                    .show_value(false)
                    .clamp_to_range(true);
                ui.add(speed_slider);
            });

            egui::Window::new("Simulation Stats")
                .show(ctx, |ui| {
                    ui.label(format!("Living Cells: {}", game.get_living_count()));
                    ui.horizontal(|ui| {
                        ui.label(format!("Total Steps: {} ", game.step_count));
                        let reset_button = ui.button("Reset");
                        if reset_button.clicked() {
                            game.step_count = 0;
                        }
                    });
                })
                .expect("Expected open window");
        });
    }
}
