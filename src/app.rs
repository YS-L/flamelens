use crate::flame::FlameGraph;
use crate::state::FlameGraphState;
use crate::view::FlameGraphView;
use std::error;
use std::sync::{Arc, Mutex};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// counter
    pub counter: u8,
    /// Flamegraph view
    pub flamegraph_view: FlameGraphView,
    /// Next flamegraph to swap in
    next_flamegraph: Arc<Mutex<Option<FlameGraph>>>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(flamegraph: FlameGraph) -> Self {
        Self {
            running: true,
            counter: 0,
            flamegraph_view: FlameGraphView::new(flamegraph),
            next_flamegraph: Arc::new(Mutex::new(None)),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        if let Some(fg) = self.next_flamegraph.lock().unwrap().take() {
            self.flamegraph_view.set_flamegraph(fg);
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn flamegraph(&self) -> &FlameGraph {
        &self.flamegraph_view.flamegraph
    }

    pub fn flamegraph_state(&self) -> &FlameGraphState {
        &self.flamegraph_view.state
    }
}
