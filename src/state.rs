use crate::flame::{StackIdentifier, ROOT_ID};

#[derive(Debug, Clone)]
pub struct FlameGraphState {
    pub selected: StackIdentifier,
    pub level_offset: usize,
    pub frame_height: Option<u16>,
    pub frame_width: Option<u16>,
    pub zoom: Option<StackIdentifier>,
}

impl Default for FlameGraphState {
    fn default() -> Self {
        Self {
            selected: ROOT_ID,
            level_offset: 0,
            frame_height: None,
            frame_width: None,
            zoom: None,
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

    pub fn set_zoom(&mut self) {
        if self.selected == ROOT_ID {
            self.unset_zoom();
        } else {
            self.zoom = Some(self.selected);
        }
    }

    pub fn unset_zoom(&mut self) {
        self.zoom = None;
    }
}
