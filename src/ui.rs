use crate::flame::{FlameGraph, StackIdentifier, StackState};
use crate::{app::App, flame::StackInfo};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame,
};
use std::collections::HashMap;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub struct FlamelensWidgetState {
    stack_states: HashMap<StackIdentifier, StackState>,
}

impl FlamelensWidgetState {
    pub fn new(fg: &FlameGraph) -> Self {
        let mut stack_states = HashMap::new();
        for stack_id in fg.get_stack_identifiers() {
            stack_states.insert(stack_id, StackState { visible: false });
        }
        Self { stack_states }
    }
}

pub struct FlamelensWidget<'a> {
    pub app: &'a App,
}

impl<'a> FlamelensWidget<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> StatefulWidget for FlamelensWidget<'a> {
    type State = FlamelensWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(2)])
            .split(area);

        // Framegraph area
        let flamegraph_area = layout[0];
        self.render_stacks(
            self.app.flamegraph.root(),
            buf,
            state,
            flamegraph_area.x,
            flamegraph_area.y,
            flamegraph_area.width,
            flamegraph_area.bottom(),
        );

        // Status bar
        let status_bar =
            Paragraph::new(self.get_status_text()).block(Block::new().borders(Borders::TOP));
        status_bar.render(layout[1], buf)
    }
}

impl<'a> FlamelensWidget<'a> {
    #[allow(clippy::too_many_arguments)]
    fn render_stacks(
        &self,
        stack: &'a StackInfo,
        buf: &mut Buffer,
        state: &mut FlamelensWidgetState,
        x: u16,
        y: u16,
        x_budget: u16,
        y_max: u16,
    ) {
        if y >= y_max || x_budget == 0 {
            state
                .stack_states
                .entry(stack.full_name.clone())
                .and_modify(|e| e.visible = false)
                .or_insert(StackState { visible: false });
            return;
        }

        state
            .stack_states
            .entry(stack.full_name.clone())
            .and_modify(|e| e.visible = true)
            .or_insert(StackState { visible: true });

        let stack_color = self.get_stack_color(stack);
        let text_color = FlamelensWidget::<'a>::get_text_color(stack_color);
        buf.set_span(
            x,
            y,
            &Span::styled(
                &format!(
                    " {:width$}",
                    stack.short_name,
                    width = x_budget.saturating_sub(1) as usize,
                ),
                Style::default().fg(text_color).bg(stack_color),
            ),
            x_budget,
        );
        let mut x_offset = 0;
        for child in &stack.children {
            let child_stack = self.app.flamegraph.get_stack(child).unwrap();
            let child_x_budget = (x_budget as f64
                * (child_stack.total_count as f64 / stack.total_count as f64))
                as u16;
            self.render_stacks(
                child_stack,
                buf,
                state,
                x + x_offset,
                y + 1,
                child_x_budget,
                y_max,
            );
            x_offset += child_x_budget;
        }
    }

    fn get_stack_color(&self, stack: &'a StackInfo) -> Color {
        if self.app.flamegraph_state.selected == stack.full_name {
            return Color::Rgb(250, 250, 250);
        }
        // Roughly based on flamegraph.pl
        fn hash_name(name: &str) -> f64 {
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            hasher.finish() as f64 / u64::MAX as f64
        }
        let v1 = hash_name(&stack.full_name);
        let v2 = hash_name(&stack.full_name.chars().rev().collect::<String>());
        let r = 205 + (50.0 * v2) as u8;
        let g = (230.0 * v1) as u8;
        let b = (55.0 * v2) as u8;
        Color::Rgb(r, g, b)
    }

    fn get_text_color(c: Color) -> Color {
        match c {
            Color::Rgb(r, g, b) => {
                let luma = 0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64;
                if luma > 128.0 {
                    Color::Black
                } else {
                    Color::Gray
                }
            }
            _ => Color::Black,
        }
    }

    fn get_status_text(&self) -> String {
        let stack = self
            .app
            .flamegraph
            .get_stack(&self.app.flamegraph_state.selected);
        match stack {
            Some(stack) => format!(
                "Current: {} [Total: {}, {:.2}%] [Self: {}, {:.2}%] {:?}",
                stack.short_name,
                stack.total_count,
                (stack.total_count as f64 / self.app.flamegraph.root().total_count as f64) * 100.0,
                stack.self_count,
                (stack.self_count as f64 / self.app.flamegraph.root().total_count as f64) * 100.0,
                stack.state,
            ),
            None => "No stack selected".to_string(),
        }
    }
}

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui-org/ratatui/tree/master/examples
    let flamelens_widget = FlamelensWidget::new(app);
    let mut flamelens_state = FlamelensWidgetState::new(&app.flamegraph);
    frame.render_stateful_widget(flamelens_widget, frame.size(), &mut flamelens_state);
    for (stack_id, stack_state) in &flamelens_state.stack_states {
        app.flamegraph.set_state(stack_id, stack_state.clone());
    }
}
