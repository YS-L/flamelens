use std::cmp::min;

use crate::{
    flame::{FlameGraph, SearchPattern, SortColumn, StackIdentifier, StackInfo, ROOT_ID},
    state::{FlameGraphState, ZoomState},
};

#[derive(Debug)]
pub struct FlameGraphView {
    pub flamegraph: FlameGraph,
    pub state: FlameGraphState,
    pub updated_at: std::time::Instant,
}

impl FlameGraphView {
    pub fn new(flamegraph: FlameGraph) -> Self {
        Self {
            flamegraph,
            state: FlameGraphState::default(),
            updated_at: std::time::Instant::now(),
        }
    }

    pub fn select_id(&mut self, stack_id: &StackIdentifier) {
        self.state.select_id(stack_id);
        if let Some(p) = self.state.search_pattern.as_ref() {
            if p.is_manual {
                return;
            }
        }
        let pattern = self.flamegraph.get_stack_short_name(stack_id);
        if let Some(pattern) = pattern {
            let search_pattern = SearchPattern::new(pattern, false, false).unwrap();
            self.set_search_pattern(search_pattern);
        }
    }

    pub fn replace_flamegraph(&mut self, mut new_flamegraph: FlameGraph) {
        self.state
            .handle_flamegraph_replacement(&self.flamegraph, &mut new_flamegraph);
        // Preserve the sort column
        new_flamegraph
            .ordered_stacks
            .set_sort_column(self.flamegraph.ordered_stacks.sorted_column);
        self.flamegraph = new_flamegraph;
        // Now the id in ZoomState points to the one in new flamegraph, but the ancestors and
        // descendants are not. Set the zoom again to update them.
        if let Some(zoom) = &self.state.zoom {
            self.set_zoom_for_id(zoom.stack_id);
        }
        self.updated_at = std::time::Instant::now();
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
            let mut children_stacks = stack
                .children
                .iter()
                .filter_map(|x| self.flamegraph.get_stack(x))
                .collect::<Vec<_>>();
            // Visit the widest child first
            children_stacks.sort_by_key(|x| x.total_count);
            let mut selected_child = None;
            for child_stack in children_stacks.iter().rev() {
                if self.is_stack_visibly_wide(child_stack, None) {
                    selected_child = Some(child_stack.id);
                    if !self.is_stack_in_view_port(child_stack) {
                        self.state.level_offset += 1;
                    }
                    break;
                }
            }
            if let Some(selected_child) = selected_child {
                self.select_id(&selected_child);
            }
        } else {
            self.state.select_root();
        }
    }

    pub fn to_parent_stack(&mut self) {
        // TODO: maybe also check parent visibility to handle resizing / edge cases
        if let Some(parent) = self
            .flamegraph
            .get_stack(&self.state.selected)
            .map(|x| x.parent)
        {
            if let Some(parent) = parent {
                if let Some(parent_stack) = self.flamegraph.get_stack(&parent) {
                    if !self.is_stack_in_view_port(parent_stack) {
                        self.state.level_offset -= 1;
                    }
                }
                self.select_id(&parent);
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

    fn is_stack_visibly_wide(&self, stack: &StackInfo, zoom_factor: Option<f64>) -> bool {
        if let Some(frame_width) = self.state.frame_width {
            let mut expected_frame_width = stack.width_factor * frame_width as f64;
            if let Some(zoom_factor) = zoom_factor {
                // Use manually specified zoom factor as the descendants / ancentors logic are
                // handled by the caller
                expected_frame_width *= zoom_factor;
            } else if let Some(zoom) = &self.state.zoom {
                let adjusted_frame_width = expected_frame_width * zoom.zoom_factor;
                // Important: Must short circuit by checking the adjusted_frame_width >= 1.0
                // condition first because the is_ancestor_or_descendant check is expensive for very
                // deep call stacks.
                if adjusted_frame_width >= 1.0 && zoom.is_ancestor_or_descendant(&stack.id) {
                    expected_frame_width = adjusted_frame_width;
                } else {
                    return false;
                }
            }
            expected_frame_width >= 1.0
        } else {
            true
        }
    }

    fn select_stack_in_view_port(&mut self) {
        if let Some(stacks) = self.flamegraph.get_stacks_at_level(self.state.level_offset) {
            for stack_id in stacks {
                if let Some(stack) = self.flamegraph.get_stack(stack_id) {
                    if self.is_stack_visibly_wide(stack, None) {
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

    pub fn get_selected_stack(&self) -> Option<&StackInfo> {
        // TODO: refactor places to call this
        self.flamegraph.get_stack(&self.state.selected)
    }

    pub fn is_root_selected(&self) -> bool {
        self.state.selected == ROOT_ID
    }

    pub fn get_next_sibling(&self, stack_id: &StackIdentifier) -> Option<StackIdentifier> {
        let stack = self.flamegraph.get_stack(stack_id)?;
        let level = self.flamegraph.get_stacks_at_level(stack.level)?;
        let level_idx = level.iter().position(|x| x == stack_id)?;
        for sibling_id in level[level_idx + 1..].iter() {
            if let Some(stack) = self.flamegraph.get_stack(sibling_id) {
                if self.is_stack_visibly_wide(stack, None) {
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
                if self.is_stack_visibly_wide(stack, None) {
                    return Some(sibling_id).cloned();
                }
            }
        }
        None
    }

    /// Get number of visible levels in the flamegraph. This prevents scrolling far down to an
    /// offset with no visible stacks as they are all too tiny.
    pub fn get_num_visible_levels(&self) -> usize {
        // Scaling factor to apply
        let zoom_factor = self
            .state
            .zoom
            .as_ref()
            .map(|z| z.zoom_factor)
            .unwrap_or(1.0);

        // Count the number of unique levels that are visible
        let starting_stack_id = if let Some(zoom) = &self.state.zoom {
            zoom.stack_id
        } else {
            ROOT_ID
        };
        self.flamegraph
            .get_descendants(&starting_stack_id)
            .iter()
            .filter_map(|id| self.flamegraph.get_stack(id))
            .filter(|stack| self.is_stack_visibly_wide(stack, Some(zoom_factor)))
            .map(|stack| stack.level)
            .max()
            .map(|x| x + 1) // e.g. if max level is 0, there is 1 level
            .unwrap_or_else(|| self.flamegraph.get_num_levels())
    }

    pub fn get_bottom_level_offset(&self) -> Option<usize> {
        self.state.frame_height.map(|frame_height| {
            self.get_num_visible_levels()
                .saturating_sub(frame_height as usize)
        })
    }

    pub fn to_previous_sibling(&mut self) {
        if let Some(stack_id) = self.get_previous_sibling(&self.state.selected) {
            self.select_id(&stack_id)
        }
    }

    pub fn to_next_sibling(&mut self) {
        if let Some(stack_id) = self.get_next_sibling(&self.state.selected) {
            self.select_id(&stack_id)
        }
    }

    pub fn to_previous_search_result(&mut self) {
        if let Some(previous_id) = self.get_previous_hit() {
            self.select_id(&previous_id);
            self.scroll_to_selected();
        }
    }

    pub fn to_next_search_result(&mut self) {
        if let Some(next_id) = self.get_next_hit() {
            self.select_id(&next_id);
            self.scroll_to_selected();
        }
    }

    fn get_next_hit(&self) -> Option<StackIdentifier> {
        // Nothing to do if not searching
        let _ = self.state.search_pattern.as_ref()?;

        // Get from the current level
        let selected_stack = self.flamegraph.get_stack(&self.state.selected)?;
        let level_stacks = self.flamegraph.get_stacks_at_level(selected_stack.level)?;
        let next_hit = self.get_next_hit_same_level(level_stacks.iter());
        if next_hit.is_some() {
            return next_hit;
        }

        // Get from the next level
        self.flamegraph.hit_ids().and_then(|hit_ids| {
            hit_ids
                .iter()
                .filter_map(|x| self.flamegraph.get_stack(x))
                .filter(|x| x.level > selected_stack.level && self.is_stack_visibly_wide(x, None))
                .map(|x| x.id)
                .next()
        })
    }

    pub fn get_previous_hit(&self) -> Option<StackIdentifier> {
        // Nothing to do if not searching
        let _ = self.state.search_pattern.as_ref()?;

        // Get from the current level
        let selected_stack = self.flamegraph.get_stack(&self.state.selected)?;
        let level_stacks = self.flamegraph.get_stacks_at_level(selected_stack.level)?;
        let hit = self.get_next_hit_same_level(level_stacks.iter().rev());
        if hit.is_some() {
            return hit;
        }

        // Get from the previous level
        self.flamegraph.hit_ids().and_then(|hit_ids| {
            hit_ids
                .iter()
                .rev()
                .filter_map(|x| self.flamegraph.get_stack(x))
                .filter(|x| x.level < selected_stack.level && self.is_stack_visibly_wide(x, None))
                .map(|x| x.id)
                .next()
        })
    }

    fn get_next_hit_same_level<'a, I>(&self, level_stacks: I) -> Option<StackIdentifier>
    where
        I: Iterator<Item = &'a StackIdentifier>,
    {
        let same_level_candidates = level_stacks
            .filter_map(|x| self.flamegraph.get_stack(x))
            .skip_while(|x| x.id != self.state.selected)
            .skip(1); // skip the selected stack
        same_level_candidates
            .filter(|x| x.hit)
            .find(|x| self.is_stack_visibly_wide(x, None))
            .map(|x| x.id)
    }

    pub fn scroll_bottom(&mut self) {
        if let Some(bottom_offset) = self.get_bottom_level_offset() {
            self.state.level_offset = bottom_offset;
            self.keep_selected_stack_in_view_port();
        }
    }

    pub fn scroll_top(&mut self) {
        self.state.level_offset = 0;
        self.keep_selected_stack_in_view_port();
    }

    pub fn scroll_to_selected(&mut self) {
        if let Some(stack) = self.get_selected_stack() {
            if !self.is_stack_in_view_port(stack) {
                self.set_level_offset(stack.level);
            }
        }
    }

    pub fn page_down(&mut self) {
        if let (Some(frame_height), Some(bottom_offset)) =
            (self.state.frame_height, self.get_bottom_level_offset())
        {
            self.set_level_offset(min(
                self.state.level_offset + frame_height as usize,
                bottom_offset,
            ));
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

    pub fn set_zoom_for_id(&mut self, stack_id: StackIdentifier) {
        if let Some(selected_stack) = self.flamegraph.get_stack(&stack_id) {
            let zoom_factor =
                self.flamegraph.total_count() as f64 / selected_stack.total_count as f64;
            let ancestors = self.flamegraph.get_ancestors(&stack_id);
            let descendants = self.flamegraph.get_descendants(&stack_id);
            if stack_id == ROOT_ID {
                self.unset_zoom();
            } else {
                let zoom = ZoomState {
                    stack_id,
                    zoom_factor,
                    ancestors,
                    descendants,
                };
                self.state.set_zoom(zoom);
            }
        }
    }

    pub fn set_zoom(&mut self) {
        self.set_zoom_for_id(self.state.selected);
    }

    pub fn unset_zoom(&mut self) {
        if let Some(zoom_stack_id) = self.state.zoom.as_ref().map(|z| z.stack_id) {
            // Restore selected to previous zoom point
            self.select_id(&zoom_stack_id);
        }
        self.state.unset_zoom();
    }

    pub fn set_search_pattern(&mut self, search_pattern: SearchPattern) {
        self.flamegraph.set_hits(&search_pattern);
        self.state.set_search_pattern(search_pattern);
    }

    pub fn unset_search_pattern(&mut self) {
        self.flamegraph.clear_hits();
        self.state.unset_search_pattern();
    }

    pub fn unset_manual_search_pattern(&mut self) {
        if let Some(p) = self.state.search_pattern.as_ref() {
            if p.is_manual {
                self.unset_search_pattern();
            }
        }
    }

    pub fn reset(&mut self) {
        self.state.select_root();
        self.state.level_offset = 0;
        self.state.unset_zoom();
        self.state.table_state.reset();
        self.unset_search_pattern();
    }

    pub fn to_next_row(&mut self) {
        let new_value = min(
            self.state.table_state.selected.saturating_add(1),
            self.flamegraph.ordered_stacks.num_rows.saturating_sub(1),
        );
        self.state.table_state.selected = new_value;
    }

    pub fn scroll_next_rows(&mut self) {
        let delta = self.state.frame_height.unwrap_or(10) as usize;
        let new_value = min(
            self.state.table_state.selected.saturating_add(delta),
            self.flamegraph.ordered_stacks.num_rows.saturating_sub(1),
        );
        self.state.table_state.selected = new_value;
        self.state.table_state.offset = new_value;
    }

    pub fn to_previous_row(&mut self) {
        let new_value = self.state.table_state.selected.saturating_sub(1);
        self.state.table_state.selected = new_value;
    }

    pub fn scroll_previous_rows(&mut self) {
        let delta = self.state.frame_height.unwrap_or(10) as usize;
        let new_value = self.state.table_state.selected.saturating_sub(delta);
        self.state.table_state.selected = new_value;
        self.state.table_state.offset = new_value;
    }

    pub fn set_sort_by_own(&mut self) {
        self.flamegraph
            .ordered_stacks
            .set_sort_column(SortColumn::Own);
    }

    pub fn set_sort_by_total(&mut self) {
        self.flamegraph
            .ordered_stacks
            .set_sort_column(SortColumn::Total);
    }

    pub fn get_selected_row_name(&mut self) -> Option<&str> {
        self.flamegraph
            .ordered_stacks
            .entries
            .get(self.state.table_state.selected)
            .map(|x| x.name.as_str())
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

    fn get_selected_short_name(view: &FlameGraphView) -> &str {
        view.flamegraph
            .get_stack_short_name(&view.state.selected)
            .unwrap()
    }

    #[test]
    fn test_get_next_sibling() {
        let content = std::fs::read_to_string("tests/data/py-spy-simple.txt").unwrap();
        let fg = FlameGraph::from_string(content, true);
        let view = FlameGraphView::new(fg);

        let result = view.get_next_sibling(&ROOT_ID);
        assert_eq!(result, None);

        let result = view.get_next_sibling(&get_id(&view, "<module> (long_running.py:25)"));
        assert_eq!(
            result.unwrap(),
            get_id(&view, "<module> (long_running.py:24)")
        );

        let result = view.get_next_sibling(&get_id(
            &view,
            "<module> (long_running.py:25);work (long_running.py:7)",
        ));
        assert_eq!(
            result.unwrap(),
            get_id(
                &view,
                "<module> (long_running.py:24);quick_work (long_running.py:17)"
            ),
        );
    }

    #[test]
    fn test_get_previous_sibling() {
        let content = std::fs::read_to_string("tests/data/py-spy-simple.txt").unwrap();
        let fg = FlameGraph::from_string(content, true);
        let view = FlameGraphView::new(fg);

        let result = view.get_previous_sibling(&ROOT_ID);
        assert_eq!(result, None);

        let result = view.get_previous_sibling(&get_id(&view, "<module> (long_running.py:24)"));
        assert_eq!(
            result.unwrap(),
            get_id(&view, "<module> (long_running.py:25)")
        );

        let result = view.get_previous_sibling(&get_id(
            &view,
            "<module> (long_running.py:24);quick_work (long_running.py:17)",
        ));
        assert_eq!(
            result.unwrap(),
            get_id(
                &view,
                "<module> (long_running.py:25);work (long_running.py:7)",
            ),
        );
    }

    #[test]
    fn test_get_next_and_previous_search_result() {
        let content = std::fs::read_to_string("tests/data/readable.txt").unwrap();
        let fg = FlameGraph::from_string(content, false);

        // No-op if no search pattern
        let mut view = FlameGraphView::new(fg);
        view.to_next_search_result();
        view.to_previous_search_result();
        assert_eq!(get_selected_short_name(&view), "all");

        // Set a search pattern
        view.set_search_pattern(
            SearchPattern::new("1-b$|2-a$|2-c$|2-e$", true, true)
                .expect("Could not create search pattern"),
        );
        assert_eq!(get_selected_short_name(&view), "all");

        // Check going to the next search result
        view.to_next_search_result();
        assert_eq!(get_selected_short_name(&view), "level1-b");

        view.to_next_search_result();
        assert_eq!(get_selected_short_name(&view), "level2-a");

        view.to_next_search_result();
        assert_eq!(get_selected_short_name(&view), "level2-c");

        view.to_next_search_result();
        assert_eq!(get_selected_short_name(&view), "level2-e");

        view.to_next_search_result();
        assert_eq!(get_selected_short_name(&view), "level2-e");

        // Check going to the previous search result
        view.to_previous_search_result();
        assert_eq!(get_selected_short_name(&view), "level2-c");

        view.to_previous_search_result();
        assert_eq!(get_selected_short_name(&view), "level2-a");

        view.to_previous_search_result();
        assert_eq!(get_selected_short_name(&view), "level1-b");

        view.to_previous_search_result();
        assert_eq!(get_selected_short_name(&view), "level1-b");
    }
}
