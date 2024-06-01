use std::cmp::min;

use crate::{
    flame::{FlameGraph, StackIdentifier, StackInfo, StackUIState},
    state::FlameGraphState,
};

#[derive(Debug)]
pub struct FlameGraphView {
    pub flamegraph: FlameGraph,
    pub state: FlameGraphState,
}

impl FlameGraphView {
    pub fn new(flamegraph: FlameGraph) -> Self {
        Self {
            flamegraph,
            state: FlameGraphState::default(),
        }
    }

    pub fn set_frame_height(&mut self, frame_height: u16) {
        self.state.frame_height = Some(frame_height);
        self.keep_selected_stack_in_view_port();
    }

    pub fn set_ui_state(&mut self, stack_id: &StackIdentifier, state: StackUIState) {
        self.flamegraph.set_ui_state(stack_id, state);
    }

    pub fn set_level_offset(&mut self, level_offset: usize) {
        let max_level_offset = self
            .flamegraph
            .get_num_levels()
            .saturating_sub(self.state.frame_height.unwrap_or(1) as usize);
        self.state.level_offset = min(level_offset, max_level_offset);
    }

    pub fn to_child_stack(&mut self) {
        if let Some(stack) = self.flamegraph.get_stack(&self.state.selected) {
            for child in &stack.children {
                if let Some(child_stack) = self.flamegraph.get_stack(child) {
                    if child_stack.is_visible() {
                        self.state.select_id(child);
                        if !self.is_stack_in_view_port(child_stack) {
                            self.state.level_offset += 1;
                        }
                        return;
                    }
                }
            }
        } else {
            self.state.select_root();
        }
    }

    pub fn to_parent_stack(&mut self) {
        // TODO: maybe also check parent visibility to handle resizing / edge cases
        if let Some(stack) = self.flamegraph.get_stack(&self.state.selected) {
            if let Some(parent) = &stack.parent {
                if let Some(parent_stack) = self.flamegraph.get_stack(parent) {
                    self.state.select_id(parent);
                    if !self.is_stack_in_view_port(parent_stack) {
                        self.state.level_offset -= 1;
                    }
                }
            }
        } else {
            self.state.select_root();
        }
    }

    fn is_stack_in_view_port(&self, stack: &StackInfo) -> bool {
        if let Some(frame_height) = self.state.frame_height {
            let min_level = self.state.level_offset;
            let max_level = min_level + frame_height as usize - 1;
            min_level <= stack.level && stack.level <= max_level
        } else {
            true
        }
    }

    fn select_stack_in_view_port(&mut self) {
        if let Some(stacks) = self.flamegraph.get_stacks_at_level(self.state.level_offset) {
            for stack_id in stacks {
                if let Some(stack) = self.flamegraph.get_stack(stack_id) {
                    if stack.is_visible() {
                        self.state.select_id(stack_id);
                        break;
                    }
                }
            }
        }
    }

    fn keep_selected_stack_in_view_port(&mut self) {
        if let Some(stack) = self.flamegraph.get_stack(&self.state.selected) {
            if !self.is_stack_in_view_port(stack) {
                self.select_stack_in_view_port();
            }
        }
    }

    pub fn to_previous_sibling(&mut self) {
        if let Some(stack) = self.flamegraph.get_previous_sibling(&self.state.selected) {
            self.state.select(stack)
        }
    }

    pub fn to_next_sibling(&mut self) {
        if let Some(stack) = self.flamegraph.get_next_sibling(&self.state.selected) {
            self.state.select(stack)
        }
    }

    pub fn scroll_bottom(&mut self) {
        if let Some(frame_height) = self.state.frame_height {
            let bottom_level_offset = self
                .flamegraph
                .get_num_levels()
                .saturating_sub(frame_height as usize);
            self.state.level_offset = bottom_level_offset;
            self.keep_selected_stack_in_view_port();
        }
    }

    pub fn scroll_top(&mut self) {
        self.state.level_offset = 0;
        self.keep_selected_stack_in_view_port();
    }

    pub fn page_down(&mut self) {
        if let Some(frame_height) = self.state.frame_height {
            self.set_level_offset(self.state.level_offset + frame_height as usize);
            self.keep_selected_stack_in_view_port();
        }
    }

    pub fn page_up(&mut self) {
        if let Some(frame_height) = self.state.frame_height {
            self.set_level_offset(
                self.state
                    .level_offset
                    .saturating_sub(frame_height as usize),
            );
            self.keep_selected_stack_in_view_port();
        }
    }
}
