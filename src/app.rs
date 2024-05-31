use crate::flame::{FlameGraph, StackInfo};
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
                if let Some(child_stack) = self.flamegraph.get_stack(child) {
                    if child_stack.is_visible() {
                        self.flamegraph_state.select_id(child);
                        if !self.is_stack_in_view_port(child_stack) {
                            self.flamegraph_state.level_offset += 1;
                        }
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
                if let Some(parent_stack) = self.flamegraph.get_stack(parent) {
                    self.flamegraph_state.select_id(parent);
                    if !self.is_stack_in_view_port(parent_stack) {
                        self.flamegraph_state.level_offset -= 1;
                    }
                }
            }
        } else {
            self.flamegraph_state.select_root();
        }
    }

    fn is_stack_in_view_port(&self, stack: &StackInfo) -> bool {
        if let Some(frame_height) = self.flamegraph_state.frame_height {
            let min_level = self.flamegraph_state.level_offset;
            let max_level = min_level + frame_height as usize - 1;
            min_level <= stack.level && stack.level <= max_level
        } else {
            true
        }
    }

    fn select_stack_in_view_port(&mut self) {
        if let Some(stacks) = self
            .flamegraph
            .get_stacks_at_level(self.flamegraph_state.level_offset)
        {
            for stack_id in stacks {
                if let Some(stack) = self.flamegraph.get_stack(stack_id) {
                    if stack.is_visible() {
                        self.flamegraph_state.select_id(stack_id);
                        break;
                    }
                }
            }
        }
    }

    fn keep_selected_stack_in_view_port(&mut self) {
        if let Some(stack) = self.flamegraph.get_stack(&self.flamegraph_state.selected) {
            if !self.is_stack_in_view_port(stack) {
                self.select_stack_in_view_port();
            }
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

    pub fn scroll_bottom(&mut self) {
        if let Some(frame_height) = self.flamegraph_state.frame_height {
            let bottom_level_offset = self
                .flamegraph
                .get_num_levels()
                .saturating_sub(frame_height as usize);
            self.flamegraph_state.level_offset = bottom_level_offset;
            self.keep_selected_stack_in_view_port();
        }
    }

    pub fn scroll_top(&mut self) {
        self.flamegraph_state.level_offset = 0;
        self.keep_selected_stack_in_view_port();
    }
}
