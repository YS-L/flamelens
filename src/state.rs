use crate::flame::{StackIdentifier, ROOT};

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
