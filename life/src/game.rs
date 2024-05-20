use rustc_hash::FxHashSet;
use std::{sync::Arc, time::Duration};
use vec2::Vector2;

use super::render::Circle;

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey, SmolStr},
    window::Window,
};

#[derive(Debug)]
pub enum LoopState {
    Playing { last_update: std::time::Instant },
    Stopped,
}

impl LoopState {
    fn new() -> Self {
        Self::Stopped
    }

    #[allow(dead_code)]
    fn should_step(&self, interval: &Duration) -> bool {
        if let Self::Playing { last_update } = self {
            last_update.elapsed() >= *interval
        } else {
            false
        }
    }

    /// Updates the `last_update` field if playing.
    /// Otherwise, this is a no-op
    fn update(&mut self, interval: &Duration) -> bool {
        if let Self::Playing { last_update } = self {
            if last_update.elapsed() >= *interval {
                *self = Self::Playing {
                    last_update: std::time::Instant::now(),
                };
                true
            } else {
                false
            }
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

#[derive(Debug)]
enum DragState {
    Dragging { prev_pos: Vector2<f64> },
    NotDragging,
}

#[derive(Debug)]
pub struct GameState {
    pan_position: Vector2<f64>,
    living_cells: FxHashSet<Vector2<i32>>,
    loop_state: LoopState,
    interval: std::time::Duration,
    window: Arc<Window>,
    mouse_position: Option<Vector2<f64>>,
    grid_size: f32,
    drag_state: DragState,
}

#[derive(Default)]
pub struct InputChanges {
    pub grid_size: Option<f32>,
    pub circles: Option<Vec<Circle>>,
    pub offset: Option<Vector2<f64>>,
}

const DEFAULT_INTERVAL: Duration = Duration::from_millis(300);
const INTERVAL_P: f32 = 1.2;

impl GameState {
    pub fn new(window: Arc<Window>, grid_size: f32) -> Self {
        Self {
            pan_position: [0.0, 0.0].into(),
            living_cells: FxHashSet::default(),
            loop_state: LoopState::new(),
            interval: DEFAULT_INTERVAL,
            window,
            mouse_position: None,
            grid_size,
            drag_state: DragState::NotDragging,
        }
    }

    pub fn toggle_playing(&mut self) {
        if self.loop_state.is_playing() {
            self.loop_state = LoopState::Stopped;
        } else {
            self.step();
            let now = std::time::Instant::now();
            self.loop_state = LoopState::Playing { last_update: now }
        }
    }

    pub fn step(&mut self) {
        use rustc_hash::FxHashMap;
        let mut adjacency_rec: FxHashMap<Vector2<i32>, u32> = FxHashMap::default();

        for i in self.living_cells.iter() {
            for j in get_adjacent(i) {
                if let Some(c) = adjacency_rec.get(&j) {
                    adjacency_rec.insert(j, *c + 1);
                } else {
                    adjacency_rec.insert(j, 1);
                }
            }
        }

        self.living_cells = adjacency_rec
            .into_iter()
            .filter(|(coords, count)| {
                3 == *count || (2 == *count && self.living_cells.contains(coords))
            })
            .map(|(coords, _count)| coords)
            .collect();
    }

    fn get_circles(&self) -> Vec<Circle> {
        self.living_cells
            .iter()
            .map(|i| to_circle(*i, self.grid_size, self.pan_position))
            .collect()
    }

    fn debug(&self) {
        let size = self.window.inner_size();
        let cursor_center_thing = if let Some(v) = self.mouse_position {
            let aspect_ratio = size.width as f64 / size.height as f64;
            let shift_amount = (size.width as f64 - size.height as f64) / 2.0;
            let x_shifted = v.x - shift_amount;
            let x_scaled = x_shifted * aspect_ratio;
            Vector2::<f64>::scale(
                Vector2::new(x_scaled, v.y),
                Vector2::new((size.width as f64).recip(), (size.height as f64).recip()),
            )
        } else {
            Vector2::<f64>::new(0.0, 0.0)
        };
    }

    fn handle_scroll(&mut self, changes: &mut InputChanges, delta: MouseScrollDelta) {
        let size = self.window.inner_size();
        let change = size.height as f32
            * 0.00005
            * match delta {
                MouseScrollDelta::LineDelta(_, n) => n,
                MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => (y * 0.2) as f32,
            };

        self.grid_size = (self.grid_size * (1.0 + change)).clamp(0.005, 1.0);
        changes.grid_size = Some(self.grid_size);

        let center = if let Some(v) = self.mouse_position {
            let aspect_ratio = size.width as f64 / size.height as f64;
            let shift_amount = (size.width as f64 - size.height as f64) / 2.0;
            let x_shifted = v.x - shift_amount;
            let x_scaled = x_shifted * aspect_ratio;
            Vector2::<f64>::scale(
                Vector2::new(x_scaled, v.y),
                Vector2::new((size.width as f64).recip(), (size.height as f64).recip()),
            ) + self.pan_position
        } else {
            Vector2::<f64>::new(0.0, 0.0)
        };

        let change = change as f64;
        let extra_offset = (center) * -change;

        // extra_offset is actually the inverse of the way pan_position works
        self.pan_position -= extra_offset;
        changes.offset = Some(self.pan_position);
        changes.circles = Some(self.get_circles());
    }

    pub fn input(&mut self, event: &WindowEvent) -> InputChanges {
        let mut changes = InputChanges::default();
        let c_char = SmolStr::new_static("c");
        let d_char = SmolStr::new_static("d");

        match event {
            // Clear the screen when "c" pressed
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Character(keystr),
                        repeat: false,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } if *keystr == c_char => {
                self.living_cells.clear();
                changes.circles = Some(Vec::new());
            }
            // Speed up
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::ArrowUp),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.interval = self.interval.div_f32(INTERVAL_P),
            // Slow down
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::ArrowDown),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.interval = self.interval.mul_f32(INTERVAL_P),

            // Forget the cursor position if it left the window
            WindowEvent::CursorLeft { .. } => {
                self.mouse_position = None;
                //self.drag_state = DragState::NotDragging;
            }

            // Debugging
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Character(keystr),
                        repeat: false,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } if *keystr == d_char => {
                self.debug();
            }

            // Zooming with scroll
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_scroll(&mut changes, *delta);
            }
            // Track the cursor
            //
            // Getting the location of the cursor in the window can only be done
            // by receiving CursorMoved events and keeping track of the last location
            // we were told of.
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Some([position.x, position.y].into());
                if let DragState::Dragging { prev_pos } = self.drag_state {
                    let pos = self.mouse_position.unwrap();
                    let size = self.window.inner_size();
                    let w = size.width as f64;
                    let h = size.height as f64;
                    let ratio = w / h;

                    let pix_diff = pos - prev_pos;
                    let norm_diff =
                        Vector2::<f64>::scale(pix_diff, Vector2::new(w.recip(), h.recip()));
                    let raw_diff = Vector2::<f64>::scale(norm_diff, Vector2::new(ratio, 1.0));
                    let diff = raw_diff; // self.grid_size as f64;

                    self.pan_position -= diff;
                    self.drag_state = DragState::Dragging { prev_pos: pos };
                    changes.circles = Some(self.get_circles());
                    changes.offset = Some(self.pan_position);
                }
            }
            // Start dragging
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                if let Some(p) = self.mouse_position {
                    self.drag_state = DragState::Dragging { prev_pos: p };
                }
            }
            // Stop dragging
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Released,
                ..
            } => {
                self.drag_state = DragState::NotDragging;
            }
            // Toggle play with space
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Space),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.toggle_playing();
                let circles = self.get_circles();
                changes.circles = Some(circles);
            }
            // Individual step with Tab
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Tab),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.step();
                let circles = self.get_circles();
                changes.circles = Some(circles);
            }
            // Cell state toggling with LMB
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if let Some(mouse_position) = self.mouse_position => {
                let size = self.window.inner_size();
                let cell_pos =
                    find_cell_num(size, mouse_position, self.pan_position, self.grid_size);

                if let Some(i) = self.living_cells.get(&cell_pos).cloned() {
                    self.living_cells.remove(&i);
                } else {
                    self.living_cells.insert(cell_pos);
                }

                let circles = self.get_circles();
                changes.circles = Some(circles)
            }
            _ => (),
        };
        changes
    }

    pub fn update(&mut self) -> Option<Vec<Circle>> {
        let should_step = self.loop_state.update(&self.interval);

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

fn to_circle(cell: Vector2<i32>, grid_size: f32, pan: Vector2<f64>) -> Circle {
    let cell = Vector2::new(
        cell.x as f32 * grid_size + grid_size / 2.0,
        cell.y as f32 * grid_size + grid_size / 2.0,
    );
    Circle {
        location: [cell.x - pan.x as f32, cell.y - (pan.y as f32)],
    }
}

fn get_adjacent(coords: &Vector2<i32>) -> [Vector2<i32>; 8] {
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
    position: Vector2<f64>,
    offset: Vector2<f64>,
    grid_size: f32,
) -> Vector2<i32> {
    let aspect_ratio = size.width as f64 / size.height as f64;
    let shift_amount = (size.width as f64 - size.height as f64) / 2.0;
    let x_shifted = position.x - shift_amount;
    let x_scaled = x_shifted * aspect_ratio;
    let position_scaled = Vector2::<f64>::scale(
        Vector2::new(x_scaled, position.y),
        Vector2::new((size.width as f64).recip(), (size.height as f64).recip()),
    );
    let final_position = (position_scaled / grid_size.into()) + (offset / grid_size as f64);
    Vector2::new(
        final_position.x.floor() as i32,
        final_position.y.floor() as i32,
    )
}
