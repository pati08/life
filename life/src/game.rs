use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    collections::VecDeque,
    sync::{
        self,
        atomic::{self, AtomicBool},
        mpsc, Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey, SmolStr},
    window::Window,
};

use super::render::Circle;
use vec2::Vector2;

/// The interval between simulation steps in auto-play mode.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(300);
/// The factor by which the interval will be multiplied or divided when
/// the player changes the simulation speed.
const INTERVAL_P: f32 = 1.2;

type LivingList = FxHashSet<Vector2<i32>>;

pub struct GameState {
    pan_position: Vector2<f64>,
    living_cells: LivingList,
    loop_state: LoopState,
    interval: std::time::Duration,
    window: Arc<Window>,
    mouse_position: Option<Vector2<f64>>,
    grid_size: f32,
    drag_state: DragState,
    input_queue: VecDeque<InputAction>,
    #[cfg(feature = "threading")]
    thread_data: ThreadData,
    living_cell_count: usize,
    pub step_count: u64,
    pub living_count_history: Vec<usize>,
    changes: StateChanges,
    /// Represents a list of times that the "player" manually toggled a cell.
    ///
    /// It is updated using `Self::step_count`, so may not be accurate if that
    /// is incorrectly manipulated.
    pub toggle_record: Vec<u64>,
}

impl GameState {
    pub fn is_playing(&self) -> bool {
        self.loop_state.is_playing()
    }

    pub fn get_living_count(&self) -> usize {
        self.living_cell_count
    }

    pub fn get_interval(&self) -> Duration {
        self.interval
    }

    pub fn set_interval(&mut self, to: Duration) {
        self.interval = to;
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

    fn get_circles(&self) -> Vec<Circle> {
        let res: Vec<Circle> = self
            .living_cells
            .iter()
            .map(|i| to_circle(*i, self.grid_size))
            .collect();
        res
    }

    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let prev_size = self.grid_size;
        let size = self.window.inner_size();
        let change = size.height as f32
            * 0.00005
            * match delta {
                MouseScrollDelta::LineDelta(_, n) => n,
                MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => (y * 0.2) as f32,
            };

        self.grid_size = (self.grid_size * (1.0 + change)).clamp(0.005, 1.0);
        self.changes.grid_size = Some(self.grid_size);

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

        let change = (self.grid_size / prev_size) as f64 - 1.0;

        // Technically the math works out to the opposite of this, but this is
        // what works with the current coordinate system.
        let extra_offset = center * change;

        // extra_offset is actually the inverse of the way pan_position works
        self.pan_position += extra_offset;
        self.changes.offset = Some(self.pan_position);
        self.changes.circles = Some(self.get_circles());
    }

    pub fn input(&mut self, event: &WindowEvent) {
        let c_char = SmolStr::new_static("c");

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
                self.clear();
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

            // Zooming with scroll
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_scroll(*delta);
            }

            // Track the cursor
            //
            // Getting the location of the cursor in the window can only be done
            // by receiving CursorMoved events and keeping track of the last location
            // we were told of.
            //
            // This block also handles panning
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
                    self.changes.offset = Some(self.pan_position);
                }
            }

            // Start panning
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                if let Some(p) = self.mouse_position {
                    self.drag_state = DragState::Dragging { prev_pos: p };
                }
            }

            // Stop panning
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Released,
                ..
            } => {
                self.drag_state = DragState::NotDragging;
            }

            // Toggle autoplay with space
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
            }

            // Cell state toggling with LMB
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if let Some(mouse_position) = self.mouse_position => {
                self.handle_left(mouse_position);
            }
            _ => (),
        };
    }

    fn clear_action(&mut self) {
        self.living_cells.clear();
        self.step_count = 0;
        self.living_count_history = vec![0];
        self.living_cell_count = 0;

        self.changes.circles = Some(Vec::new());
        self.toggle_record.clear();
    }

    fn resolve_queue(&mut self) {
        while let Some(i) = self.input_queue.pop_front() {
            match i {
                InputAction::Clear => {
                    self.clear_action();
                }
                InputAction::Toggle(cell) => {
                    self.left_action(cell);
                }
            }
        }
    }

    fn left_action(&mut self, cell_pos: Vector2<i32>) {
        if let Some(i) = self.living_cells.get(&cell_pos).cloned() {
            self.living_cells.remove(&i);
        } else {
            self.living_cells.insert(cell_pos);
        }

        let circles = self.get_circles();
        self.toggle_record.push(self.step_count);
        self.changes.circles = Some(circles);
    }
}

#[cfg(feature = "threading")]
impl GameState {
    pub fn new(window: Arc<Window>, grid_size: f32) -> Self {
        use StepThreadNotification as STN;
        let (tx, rx) = mpsc::channel();
        let condvar = Condvar::new();
        let notification = Mutex::new(StepThreadNotification::Waiting);
        let shared_thread_data = Arc::new(SharedThreadData {
            condvar,
            notification,
            computing: AtomicBool::new(false),
        });
        let join_handle = {
            let thread_data = Arc::clone(&shared_thread_data);
            std::thread::spawn(move || loop {
                let cvar = &thread_data.condvar;
                let lock = &thread_data.notification;
                let data_guard = lock.lock().unwrap();
                let mut data_guard = cvar.wait(data_guard).unwrap();
                match &*data_guard {
                    STN::Exit => break,
                    STN::Waiting => (),
                    STN::Compute(data) => {
                        thread_data
                            .computing
                            .store(true, sync::atomic::Ordering::Relaxed);
                        tx.send(compute_step(data)).unwrap();
                        *data_guard = STN::Waiting;
                    }
                }
            })
        };

        let local_thread_data = LocalThreadData { join_handle, rx };

        let thread_data = ThreadData {
            local: local_thread_data,
            shared: shared_thread_data,
        };

        Self {
            pan_position: [0.0, 0.0].into(),
            living_cells: FxHashSet::default(),
            loop_state: LoopState::new(),
            interval: DEFAULT_INTERVAL,
            window,
            mouse_position: None,
            grid_size,
            drag_state: DragState::NotDragging,
            thread_data,
            input_queue: VecDeque::new(),
            living_cell_count: 0,
            step_count: 0,
            living_count_history: vec![0],
            changes: StateChanges::default(),
            toggle_record: Vec::new(),
        }
    }

    pub fn step(&mut self) {
        if self
            .thread_data
            .shared
            .computing
            .load(atomic::Ordering::Relaxed)
        {
            return;
        }
        let mut noti_lock = self.thread_data.shared.notification.lock().unwrap();
        *noti_lock = StepThreadNotification::Compute(self.living_cells.clone());
        self.thread_data.shared.condvar.notify_all();
    }

    pub fn clear(&mut self) {
        if self
            .thread_data
            .shared
            .computing
            .load(atomic::Ordering::Relaxed)
        {
            self.input_queue.push_back(InputAction::Clear);
        } else {
            self.clear_action();
        }
    }

    fn handle_left(&mut self, mouse_position: Vector2<f64>) {
        let size = self.window.inner_size();
        let cell_pos = find_cell_num(size, mouse_position, self.pan_position, self.grid_size);
        if self
            .thread_data
            .shared
            .computing
            .load(atomic::Ordering::Relaxed)
        {
            self.input_queue.push_back(InputAction::Toggle(cell_pos));
        } else {
            self.left_action(cell_pos);
        }
    }

    pub fn update(&mut self) -> StateChanges {
        let should_step = self.loop_state.update(&self.interval);

        if should_step
            && !self
                .thread_data
                .shared
                .computing
                .load(atomic::Ordering::Relaxed)
        {
            self.step();
        }

        if let Ok(v) = self.thread_data.local.rx.try_recv() {
            self.living_cells = v;
            self.changes.circles = Some(self.get_circles());
            self.thread_data
                .shared
                .computing
                .store(false, atomic::Ordering::Relaxed);
            let mut lock = self.thread_data.shared.notification.lock().unwrap();
            *lock = StepThreadNotification::Waiting;
            self.step_count += 1;
            self.living_cell_count = self.living_cells.len();
            self.living_count_history.push(self.living_cell_count);
            drop(lock);
            self.resolve_queue();
        }

        std::mem::take(&mut self.changes)
    }
}

#[cfg(not(feature = "threading"))]
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
            input_queue: VecDeque::new(),
            living_cell_count: 0,
            step_count: 0,
            living_count_history: vec![0],
            toggle_record: Vec::new(),
        }
    }

    pub fn step(&mut self) {
        self.living_cells = compute_step(&self.living_cells);
        self.changes.circles = Some(self.get_circles());
        self.step_count += 1;
        self.living_cell_count = self.living_cells.len();
        self.living_count_history.push(self.living_cell_count);
    }

    fn clear(&mut self, changes: &mut StateChanges) {
        self.living_cells.clear();
        changes.circles = Some(Vec::new());
    }

    fn handle_left(&mut self, changes: &mut StateChanges, mouse_position: Vector2<f64>) {
        let size = self.window.inner_size();
        let cell_pos = find_cell_num(size, mouse_position, self.pan_position, self.grid_size);

        if let Some(i) = self.living_cells.get(&cell_pos).cloned() {
            self.living_cells.remove(&i);
        } else {
            self.living_cells.insert(cell_pos);
        }

        let circles = self.get_circles();
        changes.circles = Some(circles)
    }

    pub fn update(&mut self) -> StateChanges {
        let mut changes = StateChanges::default();
        let should_step = self.loop_state.update(&self.interval);

        if should_step {
            self.step(&mut changes);
        }

        self.resolve_queue(&mut changes);

        changes
    }
}

#[allow(dead_code)]
enum StepThreadNotification {
    Exit,
    Waiting,
    Compute(LivingList),
}

#[allow(dead_code)]
struct SharedThreadData {
    notification: Mutex<StepThreadNotification>,
    condvar: Condvar,
    computing: AtomicBool,
}

#[allow(dead_code)]
struct ThreadData {
    shared: Arc<SharedThreadData>,
    local: LocalThreadData,
}

#[allow(dead_code)]
struct LocalThreadData {
    join_handle: JoinHandle<()>,
    rx: mpsc::Receiver<LivingList>,
}

#[derive(Default)]
pub struct StateChanges {
    pub grid_size: Option<f32>,
    pub circles: Option<Vec<Circle>>,
    pub offset: Option<Vector2<f64>>,
}

impl StateChanges {
    fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn has_changes(&self) -> bool {
        self.grid_size.is_some() || self.circles.is_some() || self.offset.is_some()
    }
}

impl std::ops::AddAssign<StateChanges> for StateChanges {
    fn add_assign(&mut self, other: StateChanges) {
        if other.grid_size.is_some() {
            self.grid_size = other.grid_size
        };
        if other.circles.is_some() {
            self.circles = other.circles
        };
        if other.offset.is_some() {
            self.offset = other.offset
        };
    }
}

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

enum DragState {
    Dragging { prev_pos: Vector2<f64> },
    NotDragging,
}

enum InputAction {
    Clear,
    Toggle(Vector2<i32>),
}

fn to_circle(cell: Vector2<i32>, grid_size: f32) -> Circle {
    let cell = Vector2::new(
        cell.x as f32 * grid_size + grid_size / 2.0,
        cell.y as f32 * grid_size + grid_size / 2.0,
    );
    Circle {
        // location: [cell.x - pan.x as f32, cell.y - (pan.y as f32)],
        location: [cell.x, cell.y],
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

fn compute_step(prev: &LivingList) -> LivingList {
    let mut adjacency_rec: FxHashMap<Vector2<i32>, u32> = FxHashMap::default();

    for i in prev.iter() {
        for j in get_adjacent(i) {
            if let Some(c) = adjacency_rec.get(&j) {
                adjacency_rec.insert(j, *c + 1);
            } else {
                adjacency_rec.insert(j, 1);
            }
        }
    }

    adjacency_rec
        .into_iter()
        .filter(|(coords, count)| alive_rules(count, prev, coords))
        .map(|(coords, _count)| coords)
        .collect()
}

#[inline(always)]
fn alive_rules(count: &u32, prev: &LivingList, coords: &Vector2<i32>) -> bool {
    3 == *count || (2 == *count && prev.contains(coords))
}

#[cfg(feature = "threading")]
impl Drop for GameState {
    fn drop(&mut self) {
        let mut noti_lock = self.thread_data.shared.notification.lock().unwrap();
        *noti_lock = StepThreadNotification::Exit;
    }
}
