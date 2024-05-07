use std::sync::Arc;
use vec2::Vector2;

use itertools::Itertools;

use super::render::Circle;

use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
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
    pan_position: Vector2<f32>,
    living_cells: Vec<Vector2<i32>>,
    loop_state: LoopState,
    interval: std::time::Duration,
    window: Arc<Window>,
    mouse_position: Option<Vector2<f32>>,
    grid_size: f32,
}

impl GameState {
    pub fn new(window: Arc<Window>, grid_size: f32) -> Self {
        Self {
            pan_position: [0.0, 0.0].into(),
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
            .flat_map(get_adjacent)
            .dedup_with_count()
            .filter(|(count, _coords)| (2..=3).contains(count))
            .map(|(_count, coords)| coords)
            .collect()
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> Option<Vec<Circle>> {
        if let WindowEvent::CursorMoved { position, .. } = event {
            self.mouse_position = Some([position.x as f32, position.y as f32].into());
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
                        i.x as f32 - self.pan_position.x,
                        i.y as f32 - self.pan_position.y,
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
            println!("\nClick received");
            let size = self.window.inner_size();
            let cursor_position = self.mouse_position;
            let cell_pos = find_cell_num(size, cursor_position?, self.pan_position, self.grid_size);

            if let Some(i) = self.living_cells.iter().position(|e| *e == cell_pos) {
                self.living_cells.swap_remove(i);
            } else {
                self.living_cells.push(cell_pos);
            }

            let circles = self
                .living_cells
                .clone()
                .into_iter()
                .map(|i| to_circle(i, self.grid_size, self.pan_position))
                .collect();
            Some(circles)
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
                .map(|i| to_circle(i, self.grid_size, self.pan_position))
                .collect();
            Some(circles)
        } else {
            None
        }
    }
}

fn to_circle(cell: Vector2<i32>, grid_size: f32, pan: Vector2<f32>) -> Circle {
    let cell = Vector2::new(
        cell.x as f32 * grid_size + grid_size / 2.0,
        cell.y as f32 * grid_size + grid_size / 2.0,
    );
    Circle {
        location: [cell.x - pan.x, cell.y - (pan.y)],
    }
}

fn get_adjacent(coords: Vector2<i32>) -> [Vector2<i32>; 8] {
    [
        [coords.x - 1, coords.y - 1].into(),
        [coords.x - 1, coords.y + 1].into(),
        [coords.x - 1, coords.y].into(),
        [coords.x, coords.y - 1].into(),
        [coords.x, coords.y + 1].into(),
        [coords.x + 1, coords.y].into(),
        [coords.x + 1, coords.y - 1].into(),
        [coords.x + 1, coords.y + 1].into(),
    ]
}

fn find_cell_num(
    size: PhysicalSize<u32>,
    position: Vector2<f32>,
    offset: Vector2<f32>,
    grid_size: f32,
) -> Vector2<i32> {
    let (size, position, offset, grid_size) = dbg!(size, position, offset, grid_size);
    let shift_amount = (size.width as f32 - size.height as f32) / 2.0;
    let x_shifted = position.x as f32 - shift_amount;

    dbg!(shift_amount, x_shifted);
    Vector2::new(0, 0)
}
