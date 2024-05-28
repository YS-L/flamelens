use crate::{app::App, flame::StackInfo};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
    Frame,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub struct FlamelensWidget<'a> {
    pub app: &'a App,
}

impl<'a> FlamelensWidget<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for FlamelensWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_stacks(
            self.app.flamegraph.root(),
            buf,
            area.x,
            0,
            area.width,
            area.bottom(),
        );
    }
}

impl<'a> FlamelensWidget<'a> {
    fn render_stacks(
        &self,
        stack: &'a StackInfo,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        x_budget: u16,
        y_max: u16,
    ) {
        if y >= y_max {
            return;
        }
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
            self.render_stacks(child_stack, buf, x + x_offset, y + 1, child_x_budget, y_max);
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
}

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui-org/ratatui/tree/master/examples
    let flamelens_widget = FlamelensWidget::new(app);
    frame.render_widget(flamelens_widget, frame.size());
}
