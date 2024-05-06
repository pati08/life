#![feature(unboxed_closures)]
#![feature(let_chains)]
#![warn(clippy::todo)]

use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};

use std::sync::Arc;

mod render;
use render::RenderState;

mod game;
use game::GameState;

struct State<'a> {
    window: Arc<Window>,
    render_state: RenderState<'a>,
    game_state: GameState,
}

impl<'a> State<'a> {
    pub async fn new() -> (Self, EventLoop<()>) {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let window = Arc::new(window);

        let render_state = RenderState::new(window.clone(), 0.2).await;
        let game_state = GameState::new(window.clone());

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

pub async fn run() {
    let (mut state, event_loop) = State::new().await;

    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            if let Some(c) = state.game_state.update() {
                state.render_state.update_circles(|_| Some(c));
            }
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.render_state.window().id() => {
                    if let Some(c) = state.game_state.input(event) {
                        state.render_state.update_circles(|_| Some(c));
                    } else if !state.render_state.input(event) {
                        match event {
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        logical_key: Key::Named(NamedKey::Escape),
                                        ..
                                    },
                                ..
                            } => control_flow.exit(),
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
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => state.render_state.reconfigure(),
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
                }
                _ => {}
            }
        })
        .unwrap();
}
