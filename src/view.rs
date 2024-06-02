use std::cmp::min;

use crate::{
    flame::{FlameGraph, StackIdentifier, StackInfo},
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

    pub fn set_frame_width(&mut self, frame_width: u16) {
        self.state.frame_width = Some(frame_width);
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
                    if self.is_stack_visibly_wide(child_stack) {
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

    fn is_stack_visibly_wide(&self, stack: &StackInfo) -> bool {
        if let Some(frame_width) = self.state.frame_width {
            (stack.width_factor * frame_width as f64) >= 1.0
        } else {
            true
        }
    }

    fn select_stack_in_view_port(&mut self) {
        if let Some(stacks) = self.flamegraph.get_stacks_at_level(self.state.level_offset) {
            for stack_id in stacks {
                if let Some(stack) = self.flamegraph.get_stack(stack_id) {
                    if self.is_stack_visibly_wide(stack) {
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

    pub fn get_next_sibling(&self, stack_id: &StackIdentifier) -> Option<StackIdentifier> {
        let stack = self.flamegraph.get_stack(stack_id)?;
        let level = self.flamegraph.get_stacks_at_level(stack.level)?;
        let level_idx = level.iter().position(|x| x == stack_id)?;
        for sibling_id in level[level_idx + 1..].iter() {
            if let Some(stack) = self.flamegraph.get_stack(sibling_id) {
                if self.is_stack_visibly_wide(stack) {
                    return Some(sibling_id).cloned();
                }
            }
        }
        None
    }

    pub fn get_previous_sibling(&self, stack_id: &StackIdentifier) -> Option<StackIdentifier> {
        let stack = self.flamegraph.get_stack(stack_id)?;
        let level = self.flamegraph.get_stacks_at_level(stack.level)?;
        let level_idx = level.iter().position(|x| x == stack_id)?;
        for sibling_id in level[..level_idx].iter().rev() {
            if let Some(stack) = self.flamegraph.get_stack(sibling_id) {
                if self.is_stack_visibly_wide(stack) {
                    return Some(sibling_id).cloned();
                }
            }
        }
        None
    }

    pub fn to_previous_sibling(&mut self) {
        if let Some(stack_id) = self.get_previous_sibling(&self.state.selected) {
            self.state.select_id(&stack_id)
        }
    }

    pub fn to_next_sibling(&mut self) {
        if let Some(stack_id) = self.get_next_sibling(&self.state.selected) {
            self.state.select_id(&stack_id)
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

    pub fn set_zoom(&mut self) {
        self.state.set_zoom();
    }

    pub fn reset(&mut self) {
        self.state.select_root();
        self.state.level_offset = 0;
        self.state.unset_zoom();
    }
}

#[cfg(test)]
mod tests {
    use crate::flame::ROOT_ID;

    use super::*;

    fn get_id(view: &FlameGraphView, full_name: &str) -> StackIdentifier {
        view.flamegraph
            .get_stack_by_full_name(full_name)
            .unwrap()
            .id
    }

    #[test]
    fn test_get_next_sibling() {
        let fg = FlameGraph::from_file("tests/data/py-spy-simple.txt");
        let view = FlameGraphView::new(fg);

        let result = view.get_next_sibling(&ROOT_ID);
        assert_eq!(result, None);

        let result = view.get_next_sibling(&get_id(&view, "<module> (long_running.py:24)"));
        assert_eq!(
            result.unwrap(),
            get_id(&view, "<module> (long_running.py:25)")
        );

        let result = view.get_next_sibling(&get_id(
            &view,
            "<module> (long_running.py:24);quick_work (long_running.py:17)",
        ));
        assert_eq!(
            result.unwrap(),
            get_id(
                &view,
                "<module> (long_running.py:25);work (long_running.py:8)"
            ),
        );
    }

    #[test]
    fn test_get_previous_sibling() {
        let fg = FlameGraph::from_file("tests/data/py-spy-simple.txt");
        let view = FlameGraphView::new(fg);

        let result = view.get_previous_sibling(&ROOT_ID);
        assert_eq!(result, None);

        let result = view.get_previous_sibling(&get_id(&view, "<module> (long_running.py:25)"));
        assert_eq!(
            result.unwrap(),
            get_id(&view, "<module> (long_running.py:24)")
        );

        let result = view.get_previous_sibling(&get_id(
            &view,
            "<module> (long_running.py:25);work (long_running.py:8)".into(),
        ));
        assert_eq!(
            result.unwrap(),
            get_id(
                &view,
                "<module> (long_running.py:24);quick_work (long_running.py:17)"
            ),
        );
    }
}
