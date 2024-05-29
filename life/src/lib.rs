#![feature(unboxed_closures)]
#![feature(let_chains)]
#![feature(if_let_guard)]
#![warn(clippy::todo)]

use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};

use std::sync::{Arc, Mutex};

mod render;
use render::RenderState;

mod game;
use game::GameState;

struct State<'a> {
    #[allow(dead_code)]
    window: Arc<Window>,
    render_state: RenderState<'a>,
    game_state: Arc<Mutex<GameState>>,
}

/// The number of cells that will fit across the height of the window by default
const DEFAULT_GRID_SIZE: f32 = 10.0;

impl<'a> State<'a> {
    /// Create a new state and get its render loop, which it creates
    pub async fn new() -> (Self, EventLoop<()>) {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let window = Arc::new(window);

        let game_state = Arc::new(Mutex::new(GameState::new(
            window.clone(),
            DEFAULT_GRID_SIZE.recip(),
        )));

        let render_state = RenderState::new(
            window.clone(),
            DEFAULT_GRID_SIZE.recip(),
            DEFAULT_GRID_SIZE.powi(2) as u64,
            Arc::clone(&game_state),
        )
        .await;

        (
            Self {
                window,
                render_state,
                game_state,
            },
            event_loop,
        )
    }
}

/// Run the game
pub async fn run() {
    let (mut state, event_loop) = State::new().await;

    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            {
                let mut game = state.game_state.lock().unwrap();
                let game_changes = game.update();
                if let Some(c) = game_changes.circles {
                    state.render_state.update_circles(c);
                }
                if let Some(v) = game_changes.grid_size {
                    state.render_state.change_grid_size(v);
                }
                if let Some(v) = game_changes.offset {
                    let offset = vec2::Vector2::new(v.x as f32, v.y as f32);
                    state.render_state.update_offset(offset);
                }
            }
            let egui_captured = state.render_state.handle_event(&event);

            if let Event::WindowEvent {
                window_id,
                ref event,
            } = event
                && window_id == state.render_state.window().id()
            {
                if !egui_captured {
                    let mut game = state.game_state.lock().unwrap();
                    game.input(event);

                    if let WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    } = event
                    {
                        control_flow.exit();
                    }
                }

                match event {
                    WindowEvent::CloseRequested => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        surface_configured = true;
                        state.render_state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        // This tells winit that we want another frame after this one
                        state.render_state.window().request_redraw();

                        if !surface_configured {
                            return;
                        }

                        state.render_state.update();
                        match state.render_state.render() {
                            Ok(_) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                state.render_state.reconfigure()
                            }
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("OutOfMemory");
                                control_flow.exit();
                            }

                            // This happens when the a frame takes too long to present
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout")
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
        .unwrap();
}
