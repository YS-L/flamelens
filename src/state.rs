use crate::flame::{StackIdentifier, StackInfo, ROOT};

#[derive(Debug, Clone)]
pub struct FlameGraphState {
    pub selected: StackIdentifier,
}

impl Default for FlameGraphState {
    fn default() -> Self {
        Self {
            selected: ROOT.into(),
        }
    }
}

impl FlameGraphState {
    pub fn select_root(&mut self) {
        self.selected = ROOT.into();
    }

    pub fn select(&mut self, stack: &StackInfo) {
        self.selected = stack.full_name.clone();
    }

    pub fn select_id(&mut self, stack_id: &StackIdentifier) {
        self.selected = stack_id.clone();
    }
}
