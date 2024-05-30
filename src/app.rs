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

    pub fn to_child_stack(&mut self) {
        if let Some(stack) = self.flamegraph.get_stack(&self.flamegraph_state.selected) {
            for child in &stack.children {
                if let Some(stack) = self.flamegraph.get_stack(child) {
                    if stack.is_visible() {
                        self.flamegraph_state.select_id(child);
                        return;
                    }
                }
            }
        } else {
            self.flamegraph_state.select_root();
        }
    }

    pub fn to_parent_stack(&mut self) {
        // TODO: maybe also check parent visibility to handle resizing / edge cases
        if let Some(stack) = self.flamegraph.get_stack(&self.flamegraph_state.selected) {
            if let Some(parent) = &stack.parent {
                self.flamegraph_state.select_id(parent);
            }
        } else {
            self.flamegraph_state.select_root();
        }
    }

    pub fn to_previous_sibling(&mut self) {
        if let Some(stack) = self
            .flamegraph
            .get_previous_sibling(&self.flamegraph_state.selected)
        {
            self.flamegraph_state.select(stack)
        }
    }

    pub fn to_next_sibling(&mut self) {
        if let Some(stack) = self
            .flamegraph
            .get_next_sibling(&self.flamegraph_state.selected)
        {
            self.flamegraph_state.select(stack)
        }
    }
}
