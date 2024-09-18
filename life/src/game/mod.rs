use crate::platform_impl::{
    ComputeWorker, PlatformWorker, PlatformWorkerError,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::VecDeque, time::Duration};

#[cfg(target_arch = "wasm32")]
use std::rc::Rc as Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey, SmolStr},
    window::Window,
};

use self::saving::SaveGame;
use crate::game::saving::SaveData;

use super::render::Cell;
use vec2::Vector2;

pub mod saving;

/// The interval between simulation steps in auto-play mode.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(300);
/// The factor by which the interval will be multiplied or divided when
/// the player changes the simulation speed.
const INTERVAL_P: f32 = 1.2;

type LivingList = FxHashSet<Vector2<i32>>;

pub struct State {
    pan_position: Vector2<f64>,
    /// A hashset of cells (by coordinates) that are living.
    living_cells: LivingList,
    /// Timing and play information
    loop_s: LoopState,
    /// The interval between steps in auto-play mode
    interval: std::time::Duration,
    window: Arc<Window>,
    mouse_position: Option<Vector2<f64>>,
    grid_size: f32,
    /// A queue of inputs that were made during computation and therefore
    /// deferred.
    input_queue: VecDeque<QueueAction>,
    /// Synchronization between the main thread and the computing thread
    worker: Box<dyn ComputeWorker<LivingList, LivingList>>,
    living_cell_count: usize,

    /// These are for the statistics view
    pub step_count: u64,
    pub living_count_history: Vec<usize>,

    /// Changes to the state between renders are tracked here if they are
    /// relevant to the renderer so that they can be passed back on the next
    /// update.
    changes: StateChanges,

    /// Represents a list of times that the "player" manually toggled a cell.
    ///
    /// It is updated using `Self::step_count`, so may not be accurate if that
    /// is incorrectly manipulated.
    pub toggle_record: Vec<u64>,

    /// Saving data that is kept in memory during play and saved to disk when
    /// the game is closed.
    pub save_file: Option<saving::SaveData>,

    left_down_at: Option<Instant>,
    moved_since_lmb: Vector2<f64>,
}

struct DummyWorker<Args: Send, Res: Send> {
    fun: Box<dyn Fn(Args) -> Res>,
    result: Option<Res>,
    computing: bool,
}
impl<Args: Send, Res: Send> ComputeWorker<Args, Res>
    for DummyWorker<Args, Res>
{
    fn send(
        &mut self,
        data: Args,
    ) -> Result<bool, crate::platform_impl::PlatformWorkerError> {
        self.computing = true;
        if self.result.is_some() {
            Ok(false)
        } else {
            self.result = Some((self.fun)(data));
            Ok(true)
        }
    }
    fn results(
        &mut self,
    ) -> Result<Option<Res>, crate::platform_impl::PlatformWorkerError> {
        if self.result.is_some() {
            self.computing = false;
        }
        Ok(std::mem::take(&mut self.result))
    }
    fn computing(&self) -> bool {
        self.computing
    }
}

impl<Args: Send, Res: Send> DummyWorker<Args, Res> {
    #[allow(clippy::unnecessary_wraps)]
    fn new<F: Fn(Args) -> Res + Send + 'static>(
        fun: F,
    ) -> Result<Self, PlatformWorkerError> {
        Ok(Self {
            fun: Box::new(fun),
            result: None,
            computing: false,
        })
    }
}

// If you're on the web AND sharedarraybuffer is not a function
fn create_worker() -> Box<dyn ComputeWorker<LivingList, LivingList>> {
    match PlatformWorker::new(compute_step) {
        Ok(w) => Box::new(w) as Box<dyn ComputeWorker<_, _>>,
        Err(e) => {
            log::error!(
                "Failed creating worker, using dummy worker instead:\n{e:?}"
            );
            Box::new(DummyWorker::new(compute_step).unwrap())
        }
    }
}

impl State {
    pub fn is_playing(&self) -> bool {
        self.loop_s.is_playing()
    }

    /// The current number of living cells
    pub fn get_living_count(&self) -> usize {
        self.living_cell_count
    }

    pub fn get_interval(&self) -> Duration {
        self.interval
    }

    pub fn set_interval(&mut self, to: Duration) {
        self.interval = to;
    }

    /// Toggles playing. If it is starting, then it steps immediately.
    pub fn toggle_playing(&mut self) {
        if self.loop_s.is_playing() {
            self.loop_s = LoopState::Stopped;
        } else {
            self.step();
            let now = Instant::now();
            self.loop_s = LoopState::Playing { last_update: now }
        }
    }

    /// Get a vector of all the cells that should be rendered
    fn get_cells(&self) -> Vec<Cell> {
        let res: Vec<Cell> = self
            .living_cells
            .iter()
            .map(|i| to_cell(*i, self.grid_size))
            .collect();
        res
    }

    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        const PIXEL_MUL: f64 = if cfg!(target_arch = "wasm32") {
            0.2
        } else {
            3.0
        };

        let factor =
            f64::from(self.window.inner_size().height).recip() * 1400.0;

        let prev_size = self.grid_size;
        let size = self.window.inner_size();
        let change = factor
            * f64::from(size.height)
            * 0.000_005
            * match delta {
                MouseScrollDelta::LineDelta(_, n) => f64::from(n) * 12.0,
                MouseScrollDelta::PixelDelta(PhysicalPosition {
                    y, ..
                }) => y * PIXEL_MUL,
            };

        self.grid_size = (f64::from(self.grid_size) * (1.0 + change))
            .clamp(0.005, 1.0) as f32;
        self.changes.grid_size = Some(self.grid_size);

        let center = if let Some(v) = self.mouse_position {
            let aspect_ratio = f64::from(size.width) / f64::from(size.height);
            let shift_amount =
                (f64::from(size.width) - f64::from(size.height)) / 2.0;
            let x_shifted = v.x - shift_amount;
            let x_scaled = x_shifted * aspect_ratio;
            Vector2::<f64>::scale(
                Vector2::new(x_scaled, v.y),
                Vector2::new(
                    f64::from(size.width).recip(),
                    f64::from(size.height).recip(),
                ),
            ) + self.pan_position
        } else {
            Vector2::<f64>::new(0.0, 0.0)
        };

        let change = f64::from(self.grid_size / prev_size) - 1.0;

        // Technically the math works out to the opposite of this, but this is
        // what works with the current coordinate system.
        let extra_offset = center * change;

        // extra_offset is actually the inverse of the way pan_position works
        self.pan_position += extra_offset;
        self.changes.offset = Some(self.pan_position);
        self.changes.cells = Some(self.get_cells());
    }

    #[allow(clippy::too_many_lines)] // it's just barely over
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
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
                let new_pos = [position.x, position.y].into();
                if self.left_down_at.is_some() {
                    let prev_pos = self.mouse_position.unwrap();
                    let size = self.window.inner_size();
                    let w = f64::from(size.width);
                    let h = f64::from(size.height);
                    let ratio = w / h;

                    let pix_diff =
                        Vector2::from([position.x, position.y]) - prev_pos;
                    let norm_diff = Vector2::<f64>::scale(
                        pix_diff,
                        Vector2::new(w.recip(), h.recip()),
                    );
                    let diff = Vector2::<f64>::scale(
                        norm_diff,
                        Vector2::new(ratio, 1.0),
                    );

                    self.pan_position -= diff;
                    self.changes.offset = Some(self.pan_position);
                    self.moved_since_lmb +=
                        Vector2::new(diff.x.abs(), diff.y.abs());
                }
                self.mouse_position = Some(new_pos);
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

            // Track the time since LMB was pressed
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.left_down_at = Some(Instant::now());
            }

            // Toggle a cell if and only if the time since the left mouse button
            // was pressed is very small.
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(t) = self.left_down_at
                    && let Some(p) = self.mouse_position
                    && (t.elapsed() < Duration::from_millis(150)
                        && self.moved_since_lmb.magnitude() < 0.010)
                {
                    self.handle_toggle(p);
                }
                self.left_down_at = None;
                self.moved_since_lmb = Vector2::default();
            }
            _ => (),
        };
    }

    /// Clear the screen
    fn clear_action(&mut self) {
        self.living_cells.clear();
        self.step_count = 0;
        self.living_count_history = vec![0];
        self.living_cell_count = 0;

        self.changes.cells = Some(Vec::new());
        self.toggle_record.clear();
    }

    /// Resolve the input queue (`self.input_queue`)
    fn resolve_queue(&mut self) {
        while let Some(i) = self.input_queue.pop_front() {
            match i {
                QueueAction::Clear => {
                    self.clear_action();
                }
                QueueAction::Toggle(cell) => {
                    self.toggle_action(cell);
                }
                QueueAction::Load(save) => {
                    self.load_action(&save);
                }
            }
        }
    }

    /// Handle a left click by toggling the particular cell. This should not be
    /// called if the click was on the GUI.
    fn toggle_action(&mut self, cell_pos: Vector2<i32>) {
        if let Some(i) = self.living_cells.get(&cell_pos).copied() {
            self.living_cells.remove(&i);
        } else {
            self.living_cells.insert(cell_pos);
        }

        let cells = self.get_cells();
        self.toggle_record.push(self.step_count);
        self.changes.cells = Some(cells);
    }

    fn load_action(&mut self, save: &SaveGame) {
        self.clear_action();
        self.living_cells = save.living_cells();
        self.pan_position = save.pan_position();
        self.grid_size = save.grid_size();

        self.changes.cells = Some(self.get_cells());
        self.changes.grid_size = Some(self.grid_size);
        self.changes.offset = Some(self.pan_position);
    }
    pub fn new(window: Arc<Window>, grid_size: f32) -> Self {
        let save_file = SaveData::new().unwrap();
        let worker = create_worker();

        Self {
            pan_position: [0.0, 0.0].into(),
            living_cells: FxHashSet::default(),
            loop_s: LoopState::new(),
            interval: DEFAULT_INTERVAL,
            window,
            mouse_position: None,
            grid_size,
            worker,
            input_queue: VecDeque::new(),
            living_cell_count: 0,
            step_count: 0,
            living_count_history: vec![0],
            changes: StateChanges::default(),
            toggle_record: Vec::new(),
            save_file: Some(save_file),
            // #[cfg(target_arch = "wasm32")]
            // scroll_mode: Default::default(),
            left_down_at: None,
            moved_since_lmb: Vector2::default(),
        }
    }

    pub fn load_save(&mut self, save: &SaveGame) {
        // if self.worker.shared.computing.load(atomic::Ordering::Relaxed) {
        //     self.input_queue.push_back(QueueAction::Load(save.clone()));
        // } else {
        //     self.load_action(save);
        // }
        if self.worker.computing() {
            self.input_queue.push_back(QueueAction::Load(save.clone()));
        } else {
            self.load_action(save);
        }
    }

    pub fn step(&mut self) {
        if self.worker.computing() {
            return;
        }
        if let Err(e) = self.worker.send(self.living_cells.clone()) {
            log::error!("Failed sending compute request to worker: {:?}", e);
        };
    }

    pub fn clear(&mut self) {
        if self.worker.computing() {
            self.input_queue.push_back(QueueAction::Clear);
        } else {
            self.clear_action();
        }
    }

    fn handle_toggle(&mut self, mouse_position: Vector2<f64>) {
        let size = self.window.inner_size();
        let cell_pos = find_cell_num(
            size,
            mouse_position,
            self.pan_position,
            self.grid_size,
        );
        if self.worker.computing() {
            self.input_queue.push_back(QueueAction::Toggle(cell_pos));
        } else {
            self.toggle_action(cell_pos);
        }
    }

    pub fn update(&mut self) -> StateChanges {
        let should_step = self.loop_s.update(&self.interval);

        if should_step && !self.worker.computing() {
            self.step();
        }

        if let Ok(Some(v)) = self.worker.results() {
            self.living_cells = v;
            self.changes.cells = Some(self.get_cells());
            self.step_count += 1;
            self.living_cell_count = self.living_cells.len();
            self.living_count_history.push(self.living_cell_count);
            self.resolve_queue();
        }

        std::mem::take(&mut self.changes)
    }
}

#[derive(Default)]
pub struct StateChanges {
    pub grid_size: Option<f32>,
    pub cells: Option<Vec<Cell>>,
    pub offset: Option<Vector2<f64>>,
}

impl std::ops::AddAssign<StateChanges> for StateChanges {
    fn add_assign(&mut self, other: StateChanges) {
        if other.grid_size.is_some() {
            self.grid_size = other.grid_size;
        };
        if other.cells.is_some() {
            self.cells = other.cells;
        };
        if other.offset.is_some() {
            self.offset = other.offset;
        };
    }
}

pub enum LoopState {
    Playing { last_update: Instant },
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
                    last_update: Instant::now(),
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

enum QueueAction {
    Clear,
    Toggle(Vector2<i32>),
    Load(SaveGame),
}

fn to_cell(cell: Vector2<i32>, grid_size: f32) -> Cell {
    let cell = Vector2::new(
        cell.x as f32 * grid_size + grid_size / 2.0,
        cell.y as f32 * grid_size + grid_size / 2.0,
    );
    Cell {
        // location: [cell.x - pan.x as f32, cell.y - (pan.y as f32)],
        location: [cell.x, cell.y],
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
    position: Vector2<f64>,
    offset: Vector2<f64>,
    grid_size: f32,
) -> Vector2<i32> {
    let aspect_ratio = f64::from(size.width) / f64::from(size.height);
    let shift_amount = (f64::from(size.width) - f64::from(size.height)) / 2.0;
    let x_shifted = position.x - shift_amount;
    let x_scaled = x_shifted * aspect_ratio;
    let position_scaled = Vector2::<f64>::scale(
        Vector2::new(x_scaled, position.y),
        Vector2::new(
            f64::from(size.width).recip(),
            f64::from(size.height).recip(),
        ),
    );
    let final_position =
        (position_scaled / grid_size.into()) + (offset / f64::from(grid_size));
    Vector2::new(
        final_position.x.floor() as i32,
        final_position.y.floor() as i32,
    )
}

#[allow(clippy::needless_pass_by_value)]
fn compute_step(prev: LivingList) -> LivingList {
    let mut adjacency_rec: FxHashMap<Vector2<i32>, u32> = FxHashMap::default();

    for i in &prev {
        for j in get_adjacent(*i) {
            if let Some(c) = adjacency_rec.get(&j) {
                adjacency_rec.insert(j, *c + 1);
            } else {
                adjacency_rec.insert(j, 1);
            }
        }
    }

    adjacency_rec
        .into_iter()
        .filter(|(coords, count)| alive_rules(*count, &prev, *coords))
        .map(|(coords, _count)| coords)
        .collect()
}

#[inline]
fn alive_rules(count: u32, prev: &LivingList, coords: Vector2<i32>) -> bool {
    3 == count || (2 == count && prev.contains(&coords))
}
