use crate::{
    app::App,
    flame::{StackIdentifier, StackInfo},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame,
};
use std::time::Duration;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, Default)]
pub struct FlamelensWidgetState {
    frame_height: u16,
    frame_width: u16,
}

pub struct ZoomState {
    pub zoom_stack: StackIdentifier,
    pub ancestors: Vec<StackIdentifier>,
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
        let tic = std::time::Instant::now();
        let flamegraph_area = layout[0];
        let zoom_state = self
            .app
            .flamegraph_state()
            .zoom
            .as_ref()
            .map(|zoom_stack| ZoomState {
                zoom_stack: *zoom_stack,
                ancestors: self.app.flamegraph().get_ancestors(zoom_stack),
            });
        self.render_stacks(
            self.app.flamegraph().root(),
            buf,
            flamegraph_area.x,
            flamegraph_area.y,
            flamegraph_area.width as f64,
            flamegraph_area.bottom(),
            &zoom_state,
        );
        let flamegraph_render_time = tic.elapsed();

        // Status bar
        let status_bar = Paragraph::new(self.get_status_text(flamegraph_render_time))
            .block(Block::new().borders(Borders::TOP));
        status_bar.render(layout[1], buf);

        // Update widget state
        state.frame_height = flamegraph_area.height;
        state.frame_width = flamegraph_area.width;
    }
}

impl<'a> FlamelensWidget<'a> {
    #[allow(clippy::too_many_arguments)]
    fn render_stacks(
        &self,
        stack: &'a StackInfo,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        x_budget: f64,
        y_max: u16,
        zoom_state: &Option<ZoomState>,
    ) {
        let after_level_offset = stack.level >= self.app.flamegraph_state().level_offset;

        // Only render if the stack is within view port
        let effective_x_budget = x_budget as u16;
        if after_level_offset && y < y_max && effective_x_budget > 0 {
            let stack_color = self.get_stack_color(stack, zoom_state);
            let text_color = FlamelensWidget::<'a>::get_text_color(stack_color);
            buf.set_span(
                x,
                y,
                &Span::styled(
                    &format!(
                        " {:width$}",
                        stack.short_name,
                        width = effective_x_budget.saturating_sub(1) as usize,
                    ),
                    Style::default().fg(text_color).bg(stack_color),
                ),
                effective_x_budget,
            );
        }

        // TODO: probably not needed now
        // Always traverse to children to update their state even if they are out of view port
        let mut x_offset = 0;
        let zoomed_child = stack
            .children
            .iter()
            .position(|child_id| {
                if let Some(zoom_state) = zoom_state {
                    *child_id == zoom_state.zoom_stack || zoom_state.ancestors.contains(child_id)
                } else {
                    false
                }
            })
            .map(|idx| stack.children[idx]);
        for child in &stack.children {
            let child_stack = self.app.flamegraph().get_stack(child).unwrap();
            let child_x_budget = if let Some(zoomed_child_id) = zoomed_child {
                // Zoomer takes all
                if zoomed_child_id == *child {
                    x_budget
                } else {
                    0.0
                }
            } else {
                x_budget * (child_stack.total_count as f64 / stack.total_count as f64)
            };
            self.render_stacks(
                child_stack,
                buf,
                x + x_offset,
                y + if after_level_offset { 1 } else { 0 },
                child_x_budget,
                y_max,
                zoom_state,
            );
            x_offset += child_x_budget as u16;
        }
    }

    fn get_stack_color(&self, stack: &StackInfo, zoom_state: &Option<ZoomState>) -> Color {
        if self.app.flamegraph_state().selected == stack.id {
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
        let mut r = 205 + (50.0 * v2) as u8;
        let mut g = (230.0 * v1) as u8;
        let mut b = (55.0 * v2) as u8;
        if let Some(zoom_state) = zoom_state {
            if zoom_state.ancestors.contains(&stack.id) {
                r = (r as f64 / 2.5) as u8;
                g = (g as f64 / 2.5) as u8;
                b = (b as f64 / 2.5) as u8;
            }
        }
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

    fn get_status_text(&self, flamegraph_render_time: Duration) -> String {
        let stack = self
            .app
            .flamegraph()
            .get_stack(&self.app.flamegraph_state().selected);
        let root_total_count = self.app.flamegraph().root().total_count;
        let render_ms = flamegraph_render_time.as_micros() as f64 / 1000.0;
        let render_str = format!(
            "[Render: {:.2}ms / {}fps]",
            render_ms,
            if render_ms > 0.0 {
                (1000.0 / render_ms) as u32
            } else {
                0
            }
        );
        let zoom_str = if let Some(stack_id) = self.app.flamegraph_state().zoom {
            let zoom_stack = self.app.flamegraph().get_stack(&stack_id).unwrap();
            Some(format!("[Zoom: {}]", zoom_stack.short_name.clone()))
        } else {
            None
        };
        match stack {
            Some(stack) => {
                let status_text = format!(
                    "Current: {} [Total: {}, {:.2}%] [Self: {}, {:.2}%] {}",
                    stack.short_name,
                    stack.total_count,
                    (stack.total_count as f64 / root_total_count as f64) * 100.0,
                    stack.self_count,
                    (stack.self_count as f64 / root_total_count as f64) * 100.0,
                    render_str,
                );
                if let Some(zoom_str) = zoom_str {
                    format!("{} {}", status_text, zoom_str)
                } else {
                    status_text
                }
            }
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
    let mut flamelens_state = FlamelensWidgetState::default();
    frame.render_stateful_widget(flamelens_widget, frame.size(), &mut flamelens_state);
    app.flamegraph_view
        .set_frame_height(flamelens_state.frame_height);
    app.flamegraph_view
        .set_frame_width(flamelens_state.frame_width);
}
