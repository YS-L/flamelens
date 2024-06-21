use crate::flame::{FlameGraph, SearchPattern, StackIdentifier, ROOT_ID};

#[derive(Debug, Clone)]
pub struct ZoomState {
    pub stack_id: StackIdentifier,
    pub zoom_factor: f64,
}

#[derive(Debug, Clone)]
pub struct FlameGraphState {
    pub selected: StackIdentifier,
    pub level_offset: usize,
    pub frame_height: Option<u16>,
    pub frame_width: Option<u16>,
    pub zoom: Option<ZoomState>,
    pub search_pattern: Option<SearchPattern>,
    pub freeze: bool,
}

impl Default for FlameGraphState {
    fn default() -> Self {
        Self {
            selected: ROOT_ID,
            level_offset: 0,
            frame_height: None,
            frame_width: None,
            zoom: None,
            search_pattern: None,
            freeze: false,
        }
    }
}

impl FlameGraphState {
    pub fn select_root(&mut self) {
        self.selected = ROOT_ID;
    }

    pub fn select_id(&mut self, stack_id: &StackIdentifier) {
        self.selected.clone_from(stack_id);
    }

    pub fn set_zoom(&mut self, zoom_factor: f64) {
        if self.selected == ROOT_ID {
            self.unset_zoom();
        } else {
            self.zoom = Some(ZoomState {
                stack_id: self.selected,
                zoom_factor,
            });
        }
    }

    pub fn unset_zoom(&mut self) {
        self.zoom = None;
    }

    pub fn set_search_pattern(&mut self, search_pattern: SearchPattern) {
        self.search_pattern = Some(search_pattern);
    }

    pub fn unset_search_pattern(&mut self) {
        self.search_pattern = None;
    }

    pub fn toggle_freeze(&mut self) {
        self.freeze = !self.freeze;
    }

    /// Update StackIdentifiers to point to the correct ones in the new flamegraph
    pub fn handle_flamegraph_replacement(&mut self, old: &FlameGraph, new: &mut FlameGraph) {
        if self.selected != ROOT_ID {
            if let Some(new_stack_id) = Self::get_new_stack_id(&self.selected, old, new) {
                self.selected = new_stack_id;
            } else {
                self.select_root();
            }
        }
        if let Some(zoom) = &mut self.zoom {
            if let Some(new_stack_id) = Self::get_new_stack_id(&zoom.stack_id, old, new) {
                zoom.stack_id = new_stack_id;
            } else {
                self.unset_zoom();
            }
        }
        // Preserve search pattern. If expensive, can move this to next flamegraph construction
        // thread and share SearchPattern via Arc but let's keep it simple for now.
        if let Some(p) = &self.search_pattern {
            new.set_hits(p);
        }
    }

    fn get_new_stack_id(
        stack_id: &StackIdentifier,
        old: &FlameGraph,
        new: &FlameGraph,
    ) -> Option<StackIdentifier> {
        old.get_stack(stack_id).and_then(|stack| {
            new.get_stack_by_full_name(old.get_stack_full_name_from_info(stack))
                .map(|stack| stack.id)
        })
    }
}
