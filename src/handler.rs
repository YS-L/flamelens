use std::time::Instant;

use crate::{
    app::{App, AppResult, InputBuffer},
    state::ViewKind,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    if app.input_buffer.is_none() {
        let tic = Instant::now();
        handle_command(key_event, app)?;
        app.add_elapsed("handle_key_events", tic.elapsed());
        Ok(())
    } else {
        handle_input_buffer(key_event, app)
    }
}

/// Handle key events as commands
pub fn handle_command(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    let mut key_handled = handle_command_generic(key_event, app)?;
    if !key_handled {
        if app.flamegraph_state().view_kind == ViewKind::FlameGraph {
            key_handled = handle_command_flamegraph(key_event, app)?;
        } else {
            key_handled = handle_command_table(key_event, app)?;
        }
    }
    if key_handled && app.transient_message.is_some() {
        app.clear_transient_message();
    }
    Ok(())
}

pub fn handle_command_generic(key_event: KeyEvent, app: &mut App) -> AppResult<bool> {
    let mut key_handled = true;
    match key_event.code {
        // Exit application on `q`
        KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        KeyCode::Char('z') => {
            app.flamegraph_view.state.toggle_freeze();
        }
        KeyCode::Tab => {
            app.flamegraph_view.state.toggle_view_kind();
        }
        KeyCode::Char('/') => {
            app.input_buffer = Some(InputBuffer {
                buffer: tui_input::Input::new("".to_string()),
                cursor: None,
            });
        }
        KeyCode::Char('?') => {
            app.toggle_debug();
        }
        _ => {
            key_handled = false;
        }
    }
    Ok(key_handled)
}

fn handle_command_flamegraph(key_event: KeyEvent, app: &mut App) -> AppResult<bool> {
    let mut key_handled = true;
    match key_event.code {
        KeyCode::Right | KeyCode::Char('l') => {
            app.flamegraph_view.to_next_sibling();
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.flamegraph_view.to_previous_sibling();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.flamegraph_view.to_child_stack();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.flamegraph_view.to_parent_stack();
        }
        KeyCode::Char('G') => {
            app.flamegraph_view.scroll_bottom();
        }
        KeyCode::Char('g') => {
            app.flamegraph_view.scroll_top();
        }
        KeyCode::Char('f') => {
            app.flamegraph_view.page_down();
        }
        KeyCode::Char('b') => {
            app.flamegraph_view.page_up();
        }
        KeyCode::Char('n') => {
            app.flamegraph_view.to_next_search_result();
        }
        KeyCode::Char('N') => {
            app.flamegraph_view.to_previous_search_result();
        }
        KeyCode::Enter => {
            app.flamegraph_view.set_zoom();
        }
        KeyCode::Esc => {
            app.flamegraph_view.unset_zoom();
        }
        KeyCode::Char('r') => {
            app.flamegraph_view.reset();
        }
        KeyCode::Char('#') => {
            app.search_selected();
        }
        _ => {
            key_handled = false;
        }
    }
    Ok(key_handled)
}

fn handle_command_table(key_event: KeyEvent, app: &mut App) -> AppResult<bool> {
    let mut key_handled = true;
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => {
            app.flamegraph_view.to_next_row();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.flamegraph_view.to_previous_row();
        }
        KeyCode::Char('f') => {
            app.flamegraph_view.scroll_next_rows();
        }
        KeyCode::Char('b') => {
            app.flamegraph_view.scroll_previous_rows();
        }
        KeyCode::Char('1') => {
            app.flamegraph_view.set_sort_by_total();
        }
        KeyCode::Char('2') => {
            app.flamegraph_view.set_sort_by_own();
        }
        KeyCode::Char('r') => {
            app.flamegraph_view.reset();
        }
        KeyCode::Enter => {
            app.search_selected_row();
        }
        _ => {
            key_handled = false;
        }
    }
    Ok(key_handled)
}

pub fn handle_input_buffer(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    if let Some(input) = app.input_buffer.as_mut() {
        match key_event.code {
            KeyCode::Esc => {
                app.input_buffer = None;
            }
            KeyCode::Enter => {
                if input.buffer.value().is_empty() {
                    app.flamegraph_view.unset_manual_search_pattern();
                } else {
                    let re_pattern = input.buffer.value().to_string();
                    app.set_manual_search_pattern(re_pattern.as_str(), true);
                }
                app.input_buffer = None;
            }
            _ => {
                input.buffer.handle_event(&Event::Key(key_event));
            }
        }
    }
    Ok(())
}
