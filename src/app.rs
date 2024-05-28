use crate::flame::FlameGraph;
use crate::state::FlameGraphState;
use std::error;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// counter
    pub counter: u8,
    /// Flamegraph
    pub flamegraph: FlameGraph,
    /// Flamegraph state
    pub flamegraph_state: FlameGraphState,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(flamegraph: FlameGraph) -> Self {
        Self {
            flamegraph,
            running: true,
            counter: 0,
            flamegraph_state: FlameGraphState::default(),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn increment_counter(&mut self) {
        if let Some(res) = self.counter.checked_add(1) {
            self.counter = res;
        }
    }

    pub fn decrement_counter(&mut self) {
        if let Some(res) = self.counter.checked_sub(1) {
            self.counter = res;
        }
    }
}
