use crate::{
    app::{App, FlameGraphInput},
    flame::{StackIdentifier, StackInfo},
    py_spy::SamplerStatus,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
    Frame,
};
use std::time::Duration;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const SEARCH_PREFIX: &str = "Search: ";

#[derive(Debug, Clone, Default)]
pub struct FlamelensWidgetState {
    frame_height: u16,
    frame_width: u16,
    render_time: Duration,
    cursor_position: Option<(u16, u16)>,
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
        let header = Paragraph::new(self.get_header_text())
            .wrap(Wrap { trim: false })
            .block(Block::new().borders(Borders::BOTTOM | Borders::TOP));
        let header_line_count_with_borders = header.line_count(area.width) as u16 + 2;

        let status_bar = Paragraph::new(self.get_status_text())
            .wrap(Wrap { trim: true })
            .block(Block::new().borders(Borders::TOP));
        let status_line_count_with_borders = status_bar.line_count(area.width) as u16 + 1;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_line_count_with_borders),
                Constraint::Fill(1),
                Constraint::Length(status_line_count_with_borders),
            ])
            .split(area);

        // Header area
        header.render(layout[0], buf);

        // Framegraph area
        let tic = std::time::Instant::now();
        let flamegraph_area = layout[1];
        let zoom_state = self
            .app
            .flamegraph_state()
            .zoom
            .as_ref()
            .map(|zoom| ZoomState {
                zoom_stack: zoom.stack_id,
                ancestors: self.app.flamegraph().get_ancestors(&zoom.stack_id),
            });
        let re = self
            .app
            .flamegraph_state()
            .search_pattern
            .as_ref()
            .and_then(|p| {
                if p.is_manual {
                    Some(&p.re)
                } else {
                    // Don't highlight if the whole stack is expected to be matched (this is
                    // when auto-searching while navigating between stacks)
                    None
                }
            });
        self.render_stacks(
            self.app.flamegraph().root(),
            buf,
            flamegraph_area.x,
            flamegraph_area.y,
            flamegraph_area.width as f64,
            flamegraph_area.bottom(),
            &zoom_state,
            &re,
        );
        let flamegraph_render_time = tic.elapsed();

        // Status bar
        status_bar.render(layout[2], buf);

        // Update widget state
        state.frame_height = flamegraph_area.height;
        state.frame_width = flamegraph_area.width;
        state.render_time = flamegraph_render_time;
        state.cursor_position = self.get_cursor_position(layout[2]);
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
        re: &Option<&regex::Regex>,
    ) {
        let after_level_offset = stack.level >= self.app.flamegraph_state().level_offset;

        // Only render if the stack is visible
        let effective_x_budget = x_budget as u16;
        if y < y_max && effective_x_budget > 0 {
            if after_level_offset {
                let stack_color = self.get_stack_color(stack, zoom_state);
                let text_color = FlamelensWidget::<'a>::get_text_color(stack_color);
                let style = Style::default().fg(text_color).bg(stack_color);
                let line = self.get_line_for_stack(stack, effective_x_budget, style, re);
                buf.set_line(x, y, &line, effective_x_budget);
            }
        } else {
            // Can skip rendering children if the stack is already not visible
            return;
        }

        // Render children
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
                re,
            );
            x_offset += child_x_budget as u16;
        }
    }

    fn get_line_for_stack(
        &self,
        stack: &StackInfo,
        width: u16,
        style: Style,
        re: &Option<&regex::Regex>,
    ) -> Line {
        let short_name = self.app.flamegraph().get_stack_short_name_from_info(stack);

        // Empty space separator at the beginning
        let mut spans = vec![Span::styled(if width > 1 { " " } else { "." }, style)];

        // Stack name with highlighted search terms if needed
        let short_name_spans = if let (true, &Some(re)) = (stack.hit, re) {
            let mut spans: Vec<Span> = Vec::new();
            let mut matches = re.find_iter(short_name);
            for part in re.split(short_name) {
                // Non-match, regular style
                spans.push(Span::styled(part, style));
                // Match, highlighted style
                if let Some(matched) = matches.next() {
                    spans.push(Span::styled(
                        matched.as_str(),
                        style
                            .fg(Color::Rgb(225, 10, 10))
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
            spans
        } else {
            vec![Span::styled(short_name, style)]
        };
        spans.extend(short_name_spans);

        // Padding to fill the rest of the width
        let pad_length = width
            .saturating_sub(short_name.len() as u16)
            .saturating_sub(1) as usize;
        spans.push(Span::styled(
            format!("{:width$}", "", width = pad_length),
            style,
        ));

        Line::from(spans)
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
        let full_name = self.app.flamegraph().get_stack_full_name_from_info(stack);
        let v1 = hash_name(full_name);
        let v2 = hash_name(full_name);
        let mut r;
        let mut g;
        let mut b;
        if !stack.hit {
            r = 205 + (50.0 * v2) as u8;
            g = (230.0 * v1) as u8;
            b = (55.0 * v2) as u8;
        } else {
            r = 10;
            g = 35;
            b = 150;
        }
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

    fn get_header_text(&self) -> String {
        match &self.app.flamegraph_input {
            FlameGraphInput::File(path) => format!("File: {}", path),
            FlameGraphInput::Pid(pid, info) => {
                let mut out = format!("Process: {}", pid);
                if let Some(info) = info {
                    out += format!(" [{}]", info).as_str();
                }
                if let Some(state) = &self.app.sampler_state() {
                    out += match state.status {
                        SamplerStatus::Running => " [Running]".to_string(),
                        _ => " [Exited]".to_string(),
                    }
                    .as_str();
                    let duration = state.total_sampled_duration;
                    let seconds = duration.as_secs() % 60;
                    let minutes = (duration.as_secs() / 60) % 60;
                    let hours = (duration.as_secs() / 60) / 60;
                    out += format!(" [Duration: {:0>2}:{:0>2}:{:0>2}]", hours, minutes, seconds)
                        .as_str();
                    if self.app.flamegraph_state().freeze {
                        out += " [Frozen; press 'z' again to unfreeze]";
                    }
                }
                out
            }
        }
    }

    fn get_status_text(&self) -> String {
        if self.app.input_buffer.is_some() {
            self.get_status_text_buffer()
        } else {
            self.get_status_text_command()
        }
    }

    fn get_status_text_buffer(&self) -> String {
        let input_buffer = self.app.input_buffer.as_ref().unwrap();
        let status_text = format!("{}{}", SEARCH_PREFIX, input_buffer.buffer);
        status_text
    }

    fn get_cursor_position(&self, status_area: Rect) -> Option<(u16, u16)> {
        self.app.input_buffer.as_ref().map(|input_buffer| {
            (
                (input_buffer.buffer.cursor() + SEARCH_PREFIX.len()) as u16,
                status_area.bottom(),
            )
        })
    }

    fn get_status_text_command(&self) -> String {
        let stack = self
            .app
            .flamegraph()
            .get_stack(&self.app.flamegraph_state().selected);
        let root_total_count = self.app.flamegraph().root().total_count;
        let elapsed_str = format!(
            "[{}]",
            self.app
                .elapsed
                .iter()
                .map(|(k, v)| format!("{}:{:.2}ms", k, v.as_micros() as f64 / 1000.0))
                .collect::<Vec<String>>()
                .join(" ")
        );
        match stack {
            Some(stack) => {
                let zoom_total_count = self.app.flamegraph_state().zoom.as_ref().map(|zoom| {
                    self.app
                        .flamegraph()
                        .get_stack(&zoom.stack_id)
                        .unwrap()
                        .total_count
                });
                let mut status_text = format!(
                    "Function: {} {}",
                    self.app.flamegraph().get_stack_short_name_from_info(stack),
                    FlamelensWidget::get_count_stats_str(
                        None,
                        stack.total_count,
                        root_total_count,
                        zoom_total_count
                    ),
                );
                if let Some(p) = &self.app.flamegraph_state().search_pattern {
                    if let (true, Some(hit_coverage_count)) =
                        (p.is_manual, self.app.flamegraph().hit_coverage_count)
                    {
                        status_text += " ";
                        status_text += FlamelensWidget::get_count_stats_str(
                            Some(format!("Match \"{}\"", p.re.as_str()).as_str()),
                            hit_coverage_count,
                            root_total_count,
                            zoom_total_count,
                        )
                        .as_str();
                    }
                }
                if let Some(transient_message) = &self.app.transient_message {
                    status_text += format!(" [{}]", transient_message).as_str();
                }
                if self.app.debug {
                    status_text += " ";
                    status_text += elapsed_str.as_str();
                }
                status_text
            }
            None => "No stack selected".to_string(),
        }
    }

    fn get_count_stats_str(
        name: Option<&str>,
        count: u64,
        total_count: u64,
        zoomed_total_count: Option<u64>,
    ) -> String {
        format!(
            "[{}{} samples, {:.2}% of all{}]",
            name.map(|n| format!("{}: ", n)).unwrap_or_default(),
            count,
            (count as f64 / total_count as f64) * 100.0,
            if let Some(zoomed_total_count) = zoomed_total_count {
                format!(
                    ", {:.2}% of zoomed",
                    (count as f64 / zoomed_total_count as f64) * 100.0
                )
            } else {
                "".to_string()
            }
        )
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
    app.add_elapsed("render", flamelens_state.render_time);
    if let Some(input_buffer) = &mut app.input_buffer {
        input_buffer.cursor = flamelens_state.cursor_position;
    }
}
