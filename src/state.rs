use crate::flame::{StackIdentifier, StackInfo, ROOT};

#[derive(Debug, Clone)]
pub struct FlameGraphState {
    pub selected: StackIdentifier,
    pub level_offset: usize,
    pub frame_height: Option<u16>,
    pub frame_width: Option<u16>,
}

impl Default for FlameGraphState {
    fn default() -> Self {
        Self {
            selected: ROOT.into(),
            level_offset: 0,
            frame_height: None,
            frame_width: None,
        }
    }
}

impl FlameGraphState {
    pub fn select_root(&mut self) {
        self.selected = ROOT.into();
    }

    pub fn select(&mut self, stack: &StackInfo) {
        self.selected.clone_from(&stack.full_name);
    }

    pub fn select_id(&mut self, stack_id: &StackIdentifier) {
        self.selected.clone_from(stack_id);
    }
}
