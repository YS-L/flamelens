#[cfg(feature = "python")]
use crate::py_spy::SamplerStatus;
use crate::{
    app::{App, FlameGraphInput},
    flame::{StackIdentifier, StackInfo},
    state::{SortColumn, ViewKind},
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        block::Position, Block, Borders, Paragraph, Row, StatefulWidget, Table, TableState, Widget,
        Wrap,
    },
    Frame,
};
use std::time::Duration;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const SEARCH_PREFIX: &str = "Search: ";
const COLOR_SELECTED_STACK: Color = Color::Rgb(250, 250, 250);
const COLOR_SELECTED_BACKGROUND: Color = COLOR_SELECTED_STACK;
const COLOR_MATCHED_BACKGROUND: Color = Color::Rgb(10, 35, 150);
const COLOR_TABLE_SELECTED_ROW: Color = Color::Rgb(65, 65, 65);

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
        self.render_all(area, buf, state);
    }
}

impl<'a> FlamelensWidget<'a> {
    fn render_all(self, area: Rect, buf: &mut Buffer, state: &mut FlamelensWidgetState) {
        let header_bottom_title = self.get_header_bottom_title();
        let header_text = Text::from(vec![self.get_header_text(area.width), Line::from("")]);
        let header = Paragraph::new(header_text)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::BOTTOM)
                    .title_position(Position::Bottom)
                    .title(header_bottom_title)
                    .title_alignment(Alignment::Center),
            );
        let header_line_count_with_borders = header.line_count(area.width) as u16 + 1;

        let mut status_bar =
            Paragraph::new(self.get_status_text(area.width)).wrap(Wrap { trim: true });
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

        // Main area for flamegraph / top view
        let tic = std::time::Instant::now();
        let main_area = layout[1];
        let has_more_rows_to_render =
            if self.app.flamegraph_state().view_kind == ViewKind::FlameGraph {
                self.render_flamegraph(main_area, buf)
            } else {
                self.render_table(main_area, buf);
                false
            };
        let flamegraph_render_time = tic.elapsed();

        // More rows indicator
        let mut status_bar_block = Block::new().borders(Borders::TOP);
        if has_more_rows_to_render {
            status_bar_block = status_bar_block
                .title(" More ▾ (press f to scroll) ")
                .title_alignment(Alignment::Center);
        }
        status_bar = status_bar.block(status_bar_block);

        // Status bar
        status_bar.render(layout[2], buf);

        // Update widget state
        state.frame_height = main_area.height;
        state.frame_width = main_area.width;
        state.render_time = flamegraph_render_time;
        state.cursor_position = self.get_cursor_position(layout[2]);
    }

    fn render_flamegraph(&self, area: Rect, buf: &mut Buffer) -> bool {
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
        let has_more_rows_to_render = self.render_stacks(
            self.app.flamegraph().root(),
            buf,
            area.x,
            area.y,
            area.width as f64,
            area.bottom(),
            &zoom_state,
            &re,
        );
        has_more_rows_to_render
    }

    fn render_table(&self, area: Rect, buf: &mut Buffer) {
        let ordered_stacks_table = self.get_ordered_stacks_table();
        let mut table_state =
            TableState::default().with_selected(self.app.flamegraph_state().table_state.selected);
        StatefulWidget::render(ordered_stacks_table, area, buf, &mut table_state);
    }

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
    ) -> bool {
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
            let has_more_rows_to_render = (y >= y_max) && effective_x_budget > 0;
            return has_more_rows_to_render;
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

        let mut has_more_rows_to_render = false;
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
            has_more_rows_to_render |= self.render_stacks(
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

        has_more_rows_to_render
    }

    fn get_ordered_stacks_table(&self) -> Table {
        let add_sorted_indicator = |label: &str, sort_column: SortColumn| {
            let suffix = if sort_column == self.app.flamegraph_state().table_state.sort_column {
                '▼'
            } else if sort_column == SortColumn::Total {
                '1'
            } else {
                '2'
            };
            format!("{} [{}]", label, suffix)
        };
        let header = Row::new(vec![
            add_sorted_indicator("Total", SortColumn::Total),
            add_sorted_indicator("Own", SortColumn::Own),
            "Name".to_string(),
        ])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        );
        let counts = if self.app.flamegraph_state().table_state.sort_column == SortColumn::Total {
            &self.app.flamegraph().ordered_stacks.by_total_count
        } else {
            &self.app.flamegraph().ordered_stacks.by_own_count
        };
        let mut rows = vec![];
        let total_count = self.app.flamegraph().total_count();
        let mut total_max_width: u16 = 0;
        let mut own_max_width: u16 = 0;

        fn format_count(count: u64, total_count: u64) -> String {
            format!(
                "{} ({:.2}%)  ",
                count,
                100.0 * count as f64 / total_count as f64
            )
        }

        for (name, count) in counts.iter() {
            let total_formatted = format_count(count.total, total_count);
            let own_formatted = format_count(count.own, total_count);
            total_max_width = total_max_width.max(total_formatted.len() as u16);
            own_max_width = own_max_width.max(own_formatted.len() as u16);
            rows.push(Row::new(vec![
                total_formatted,
                own_formatted,
                name.to_string(),
            ]));
        }
        let widths = [
            Constraint::Max(total_max_width),
            Constraint::Max(own_max_width),
            Constraint::Fill(1),
        ];
        Table::new(rows, widths)
            .header(header)
            .highlight_style(Style::default().bg(COLOR_TABLE_SELECTED_ROW))
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
            return COLOR_SELECTED_STACK;
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
        } else if let Color::Rgb(r_, g_, b_) = COLOR_MATCHED_BACKGROUND {
            r = r_;
            g = g_;
            b = b_;
        } else {
            unreachable!();
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

    fn get_style_from_bg(background_color: Color) -> Style {
        Style::default()
            .bg(background_color)
            .fg(FlamelensWidget::get_text_color(background_color))
    }

    fn get_header_bottom_title(&self) -> Line {
        let mut header_bottom_title_spans = vec![Span::from(" ")];

        fn _get_view_kind_span(
            label: &str,
            view_kind: ViewKind,
            current_view_kind: ViewKind,
        ) -> Span {
            let (content, style) = if view_kind == current_view_kind {
                (format!("[{}]", label), Style::default().bold().red())
            } else {
                (label.to_string(), Style::default())
            };
            Span::styled(content, style)
        }

        header_bottom_title_spans.push(_get_view_kind_span(
            "Flamegraph",
            ViewKind::FlameGraph,
            self.app.flamegraph_state().view_kind,
        ));
        header_bottom_title_spans.push(Span::from(" | "));
        header_bottom_title_spans.push(_get_view_kind_span(
            "Top Functions",
            ViewKind::Table,
            self.app.flamegraph_state().view_kind,
        ));
        header_bottom_title_spans.push(Span::from(" (press TAB to switch) "));
        Line::from(header_bottom_title_spans)
    }

    fn get_header_text(&self, _width: u16) -> Line {
        let header_text = match &self.app.flamegraph_input {
            FlameGraphInput::File(path) => format!("File: {}", path),
            FlameGraphInput::Pid(pid, info) => {
                let mut out = format!("Process: {}", pid);
                if let Some(info) = info {
                    out += format!(" [{}]", info).as_str();
                }
                #[cfg(feature = "python")]
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
        };
        Line::from(header_text).style(Style::default().bold())
    }

    fn get_status_text(&self, width: u16) -> Vec<Line> {
        if self.app.input_buffer.is_some() {
            self.get_status_text_buffer()
        } else {
            self.get_status_text_command(width)
        }
    }

    fn get_status_text_buffer(&self) -> Vec<Line> {
        let input_buffer = self.app.input_buffer.as_ref().unwrap();
        let status_text = format!("{}{}", SEARCH_PREFIX, input_buffer.buffer);
        vec![Line::from(status_text)]
    }

    fn get_cursor_position(&self, status_area: Rect) -> Option<(u16, u16)> {
        self.app.input_buffer.as_ref().map(|input_buffer| {
            (
                (input_buffer.buffer.cursor() + SEARCH_PREFIX.len()) as u16,
                status_area.bottom(),
            )
        })
    }

    fn get_status_text_command(&self, width: u16) -> Vec<Line> {
        let stack = self
            .app
            .flamegraph()
            .get_stack(&self.app.flamegraph_state().selected);
        let root_total_count = self.app.flamegraph().root().total_count;
        let mut lines = vec![];
        match stack {
            Some(stack) => {
                let zoom_total_count = self.app.flamegraph_state().zoom.as_ref().map(|zoom| {
                    self.app
                        .flamegraph()
                        .get_stack(&zoom.stack_id)
                        .unwrap()
                        .total_count
                });
                if let Some(p) = &self.app.flamegraph_state().search_pattern {
                    if let (true, Some(hit_coverage_count)) =
                        (p.is_manual, self.app.flamegraph().hit_coverage_count())
                    {
                        let match_text = format!(
                            "Match: \"{}\" {}",
                            p.re.as_str(),
                            FlamelensWidget::get_count_stats_str(
                                None,
                                hit_coverage_count,
                                root_total_count,
                                zoom_total_count,
                            )
                        );
                        let match_text = format!("{:width$}", match_text, width = width as usize,);
                        lines.push(
                            Line::from(match_text).style(FlamelensWidget::get_style_from_bg(
                                COLOR_MATCHED_BACKGROUND,
                            )),
                        );
                    }
                }
                let selected_text = format!(
                    "Selected: {} {}",
                    self.app.flamegraph().get_stack_short_name_from_info(stack),
                    FlamelensWidget::get_count_stats_str(
                        None,
                        stack.total_count,
                        root_total_count,
                        zoom_total_count
                    ),
                );
                let status_text = format!("{:width$}", selected_text, width = width as usize,);
                lines.push(
                    Line::from(status_text).style(FlamelensWidget::get_style_from_bg(
                        COLOR_SELECTED_BACKGROUND,
                    )),
                );
                if self.app.debug {
                    let elapsed_str = format!(
                        "Debug: {}",
                        self.app
                            .elapsed
                            .iter()
                            .map(|(k, v)| format!("{}:{:.2}ms", k, v.as_micros() as f64 / 1000.0))
                            .collect::<Vec<String>>()
                            .join(" ")
                    );
                    lines.push(Line::from(elapsed_str));
                }
                if let Some(transient_message) = &self.app.transient_message {
                    lines.push(Line::from(transient_message.as_str()));
                }
                lines
            }
            None => vec![Line::from("No stack selected")],
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
