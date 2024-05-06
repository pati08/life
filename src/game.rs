use std::sync::Arc;

use itertools::Itertools;

use super::render::Circle;

use winit::{
    dpi::PhysicalPosition, event::{ElementState, KeyEvent, MouseButton, WindowEvent}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

pub enum LoopState {
    Playing {
        last_update: std::time::Instant,
        interval: std::time::Duration,
    },
    Stopped,
}

impl LoopState {
    fn new() -> Self {
        Self::Stopped
    }

    fn should_step(&self) -> bool {
        if let Self::Playing {
            last_update,
            interval,
        } = self
        {
            last_update.elapsed() >= *interval
        } else {
            false
        }
    }

    fn is_playing(&self) -> bool {
        match self {
            Self::Stopped => false,
            Self::Playing { .. } => true,
        }
    }
}

pub struct GameState {
    pan_position: [f32; 2],
    living_cells: Vec<[i32; 2]>,
    loop_state: LoopState,
    interval: std::time::Duration,
    window: Arc<Window>,
    mouse_position: Option<[f32; 2]>,
    grid_size: f32,
}

impl GameState {
    pub fn new(window: Arc<Window>, grid_size: f32) -> Self {
        Self {
            pan_position: [0.0, 0.0],
            living_cells: Vec::new(),
            loop_state: LoopState::new(),
            interval: std::time::Duration::from_millis(300),
            window,
            mouse_position: None,
            grid_size,
        }
    }

    pub fn toggle_playing(&mut self) {
        if self.loop_state.is_playing() {
            self.loop_state = LoopState::Stopped;
        } else {
            self.step();
            let now = std::time::Instant::now();
            self.loop_state = LoopState::Playing {
                interval: self.interval,
                last_update: now,
            }
        }
    }

    pub fn step(&mut self) {
        // TODO: figure out how to do this without the clone
        self.living_cells = self
            .living_cells
            .clone()
            .into_iter()
            .map(get_adjacent)
            .flatten()
            .dedup_with_count()
            .filter_map(|(count, coords)| (count >= 2 && count <= 3).then(move || coords))
            .collect()
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> Option<Vec<Circle>> {
        if let WindowEvent::CursorMoved { position, .. } = event {
            self.mouse_position = Some([position.x as f32, position.y as f32]);
        }
        if let WindowEvent::CursorLeft { .. } = event {
            self.mouse_position = None;
        }

        if let WindowEvent::KeyboardInput { event, .. } = event
            && let KeyEvent { physical_key, .. } = event
            && let PhysicalKey::Code(KeyCode::Space) = physical_key
        {
            self.toggle_playing();
            let circles = self
                .living_cells
                .clone()
                .into_iter()
                .map(|i| Circle {
                    location: [
                        i[0] as f32 - self.pan_position[0],
                        i[1] as f32 - self.pan_position[1],
                    ],
                })
                .collect();
            Some(circles)
        } else if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = event
        {
            let size = self.window.inner_size();
            let cursor_position = self.mouse_position;
            todo!()
        } else {
            None
        }
    }

    pub fn update(&mut self) -> Option<Vec<Circle>> {
        let should_step = if let LoopState::Playing {
            ref mut last_update,
            ref interval,
        } = self.loop_state
            && last_update.elapsed() >= *interval
        {
            *last_update = std::time::Instant::now();
            true
        } else {
            false
        };

        if should_step {
            self.step();
            let circles = self
                .living_cells
                .clone()
                .into_iter()
                .map(|i| Circle {
                    location: [
                        i[0] as f32 - self.pan_position[0],
                        i[1] as f32 - self.pan_position[1],
                    ],
                })
                .collect();
            Some(circles)
        } else {
            None
        }
    }
}

fn get_adjacent(coords: [i32; 2]) -> [[i32; 2]; 8] {
    [
        [coords[0] - 1, coords[1] - 1],
        [coords[0] - 1, coords[1] + 1],
        [coords[0] - 1, coords[1]],
        [coords[0], coords[1] - 1],
        [coords[0], coords[1] + 1],
        [coords[0] + 1, coords[1]],
        [coords[0] + 1, coords[1] - 1],
        [coords[0] + 1, coords[1] + 1],
    ]
}

fn find_cell_num(size: PhysicalPosition<u32>, position: , offset: [f32; 2], grid_size: f32) -> [u32; 2] {
    todo!()
}




