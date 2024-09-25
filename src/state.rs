use crate::flame::{FlameGraph, SearchPattern, StackIdentifier, ROOT_ID};

#[derive(Debug, Clone)]
pub struct ZoomState {
    pub stack_id: StackIdentifier,
    pub ancestors: Vec<StackIdentifier>,
    pub descendants: Vec<StackIdentifier>,
    pub zoom_factor: f64,
}

impl ZoomState {
    pub fn is_ancestor_or_descendant(&self, stack_id: &StackIdentifier) -> bool {
        self.ancestors.contains(stack_id) || self.descendants.contains(stack_id)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ViewKind {
    FlameGraph,
    Table,
}

#[derive(Default, Debug, Clone)]
pub struct TableState {
    pub selected: usize,
    pub offset: usize,
}

impl TableState {
    pub fn reset(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }
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
    pub view_kind: ViewKind,
    pub table_state: TableState,
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
            view_kind: ViewKind::FlameGraph,
            table_state: TableState::default(),
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

    pub fn set_zoom(&mut self, zoom: ZoomState) {
        self.zoom = Some(zoom);
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

    pub fn toggle_view_kind(&mut self) {
        self.view_kind = match self.view_kind {
            ViewKind::FlameGraph => ViewKind::Table,
            ViewKind::Table => ViewKind::FlameGraph,
        };
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
