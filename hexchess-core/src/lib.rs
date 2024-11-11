#![feature(unboxed_closures)]
#![feature(let_chains)]
#![feature(if_let_guard)]
#![deny(clippy::todo)]
#![warn(clippy::pedantic)]
// When I cast like this, I always keep in mind the precision or truncation
// issues anyway
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]

use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::rc::Rc as Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
use std::sync::Mutex;

mod platform_impl;

mod render;

mod game;

struct State<'a> {
    #[allow(dead_code)]
    window: Arc<Window>,
    render: render::State<'a>,
    game: Arc<Mutex<game::State>>,
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

        let game_state = Arc::new(Mutex::new(game::State::new(
            window.clone(),
            DEFAULT_GRID_SIZE.recip(),
        )));

        let render_state = render::State::new(
            window.clone(),
            DEFAULT_GRID_SIZE.recip(),
            DEFAULT_GRID_SIZE.powi(2) as u64,
            Arc::clone(&game_state),
        )
        .await;

        (
            Self {
                window,
                render: render_state,
                game: game_state,
            },
            event_loop,
        )
    }
}

/// Run the game
///
/// # Panics
/// This function panics only when encountering states that cannot be recovered
/// from, such as a poisoned mutex on the game state.
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
                let mut game = state.game.lock().unwrap();
                let game_changes = game.update();
                if let Some(c) = game_changes.cells {
                    state.render.update_cells(c);
                }
                if let Some(v) = game_changes.grid_size {
                    state.render.change_grid_size(v);
                }
                if let Some(v) = game_changes.offset {
                    let offset = vec2::Vector2::new(v.x as f32, v.y as f32);
                    state.render.update_offset(offset);
                }
            }

            let egui_captured = state.render.handle_event(&event);

            // Pass memory warnings to the log output
            if let Event::MemoryWarning = event {
                log::warn!("Warning: low memory");
            };

            if let Event::WindowEvent {
                window_id,
                ref event,
            } = event
                && window_id == state.render.window().id()
            {
                // If the gui didn't capture the event, then hand it to the game
                // or, if it was the escape key, exit
                if !egui_captured {
                    let mut game = state.game.lock().unwrap();
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
                        #[cfg(not(target_arch = "wasm32"))]
                        control_flow.exit();
                    }
                }

                match event {
                    WindowEvent::CloseRequested => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        surface_configured = true;
                        state.render.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        // This tells winit that we want another frame after this one
                        state.render.window().request_redraw();

                        // We can't draw if the surface is not properly configured
                        if !surface_configured {
                            return;
                        }

                        match state.render.render() {
                            Ok(()) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(
                                wgpu::SurfaceError::Lost
                                | wgpu::SurfaceError::Outdated,
                            ) => state.render.reconfigure(),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("OutOfMemory");
                                control_flow.exit();
                            }

                            // This happens when the a frame takes too long to present
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout");
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
        .unwrap();
}
