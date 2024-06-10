use egui::{Color32, Context, Id, RichText, Slider, TextEdit, TexturesDelta, Ui};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use ::egui::FontDefinitions;
use egui_plot::{Line, Plot, VLine};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::Device;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event},
};

use crate::game::saving::SaveGame;
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
    /// Handle a winit event. Returns `true` if the event is captured by the gui.
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
        let app = Gui { game_state, new_save_name: String::new() };
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
        // End the UI frame. We could now handle the output and draw the UI with
        // the backend.
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

/// The graphical user interface's persisted state, which contains everything
/// it needs to render to an `Egui::Context`. 
struct Gui {
    game_state: Arc<Mutex<GameState>>,
    new_save_name: String,
}

impl Gui {
    const PLAYING_TEXT: &'static str = "Playing \u{23F5}";
    const NOT_PLAYING_TEXT: &'static str = "Stopped \u{23F8}";

    /// Render the top panel's UI elements within some `Ui`.
    fn top_panel_ui(&mut self, ui: &mut Ui) {
        let mut game = self.game_state.lock().unwrap();
        ui.horizontal(|ui| {
            let reset_button =
                ui.button(
                    RichText::new("RESET GAME")
                    .color(Color32::RED)
                    .strong()
                );
            if reset_button.clicked() {
                game.clear();
            }
            let button_text = if game.is_playing() {
                Self::PLAYING_TEXT
            } else {
                Self::NOT_PLAYING_TEXT
            };
            let play_button = ui.button(button_text);
            if play_button.clicked() {
                game.toggle_playing();
            }
            let speed_get_set = |set: Option<f64>| {
                if let Some(v) = set {
                    game.set_interval(std::time::Duration::from_secs_f64(v));
                }
                game.get_interval().as_secs_f64()
            };
            ui.label("Speed: ");
            let speed_slider =
                Slider::from_get_set(1f64..=0.01f64, speed_get_set)
                .show_value(false)
                .clamp_to_range(true);
            ui.add(speed_slider);
        });
    }

    /// Render the simulation statistics within some `Ui`.
    fn simulation_stats_ui(&mut self, ui: &mut Ui) {
        let mut game = self.game_state.lock().unwrap();
        ui.label(format!("Living Cells: {}", game.get_living_count()));
        ui.vertical_centered(|ui| {
            let reset_button = ui.button(
                RichText::new("Reset stats and graph")
                    .color(Color32::RED)
                    .strong(),
            );
            if reset_button.clicked() {
                game.step_count = 0;
                game.living_count_history = vec![0];
                game.toggle_record.clear();
            }
        });
        ui.label(format!("Total Steps: {} ", game.step_count));
        let line_values = game
            .living_count_history
            .iter()
            .enumerate()
            .map(|(i, j)| [i as f64, *j as f64])
            .collect::<Vec<[f64; 2]>>();
        let line = Line::new(line_values);
        Plot::new("living_cell_count_plot")
            .show_axes(false) // This was causing annoying margins
            .show(ui, |plot_ui| {
                plot_ui.line(line);
                for i in game.toggle_record.iter() {
                    if *i != 0 {
                        plot_ui
                            .vline(VLine::new(*i as f64).color(Color32::LIGHT_GREEN));
                    }
                }
            });
    }

    /// Render the interface for saving and loading within some `Ui`.
    fn saving_ui(&mut self, ui: &mut Ui) {
        let mut game = self.game_state.lock().unwrap();

        let save_file = game.save_file.as_ref().expect("Expected save file.");
        let save_count = save_file.save_count();
        for (i, save) in save_file.saves_iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(&save.name);
                ui.label(&save.created.format("%B %e").to_string());
                if ui.button("Load").clicked() {
                    game.load_save(&save);
                }
                if ui.button(RichText::new("Delete").color(Color32::RED)).clicked() {
                    let _ = game.save_file.as_mut().unwrap().delete_save(i);
                }
            });
            if i == save_count - 1 {
                ui.separator();
            }
        }
        TextEdit::singleline(&mut self.new_save_name)
            .hint_text("Save Name")
            .show(ui);
        if ui.button("Save").clicked() && !self.new_save_name.is_empty() {
            let new_save = SaveGame::new(
                game.deref_mut(),
                std::mem::take(&mut self.new_save_name)
                );
            game.save_file.as_mut().unwrap().add_save(new_save);
        }
    }

    /// Render the interface to an `Egui::Context`.
    fn ui(&mut self, ctx: &Context) {
        // Top panel with some controls
        egui::containers::panel::TopBottomPanel::top(Id::new("top_panel"))
            .show(ctx, |ui| {
                self.top_panel_ui(ui);
            });
        // Collapsible window with statistics shown
        egui::Window::new("Simulation Stats")
            .show(ctx, |ui| {
                self.simulation_stats_ui(ui);
            })
            .expect("Expected open window");

        // Collapsible window with a game saving menu.
        egui::Window::new("Game Saves")
            .show(ctx, |ui| {
                self.saving_ui(ui);
            });
    }
}
