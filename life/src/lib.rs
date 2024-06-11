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

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
    /// Create a new state and get its accompanying event loop
    pub async fn new() -> (Self, EventLoop<()>) {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let window = Arc::new(window);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("game")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
            //window.request_inner_size(PhysicalSize::new(800, 600)).unwrap();
        }

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
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Warn)
            .expect("logging init failed");
    }
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let (mut state, event_loop) = State::new().await;

    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            // Update the game state. TODO: move this logic into rendering
            {
                let mut game = state.game_state.lock().unwrap();
                let game_changes = game.update();
                if let Some(c) = game_changes.cells {
                    state.render_state.update_cells(c);
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

            // Pass memory warnings to the log output
            if let Event::MemoryWarning = event {
                log::warn!("Warning: low memory");
            };

            if let Event::WindowEvent {
                window_id,
                ref event,
            } = event
                && window_id == state.render_state.window().id()
            {
                // If the gui didn't capture the event, then hand it to the game
                // or, if it was the escape key, exit
                if !egui_captured {
                    let mut game = state.game_state.lock().unwrap();
                    game.handle_window_event(event);

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

                        // We can't draw if the surface is not properly configured
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
