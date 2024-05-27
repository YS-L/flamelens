use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
    Frame,
};

use crate::{app::App, flame::StackInfo};

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
        buf.set_span(
            x,
            y,
            &Span::styled(
                &format!(
                    "{:width$}",
                    stack.short_name,
                    width = x_budget.saturating_sub(1) as usize
                ),
                Style::default().fg(Color::White).bg(Color::Red),
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
