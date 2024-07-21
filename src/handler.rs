use std::time::Instant;

use crate::app::{App, AppResult, InputBuffer};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    if app.input_buffer.is_none() {
        handle_command(key_event, app)
    } else {
        handle_input_buffer(key_event, app)
    }
}

/// Handle key events as commands
pub fn handle_command(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    let tic = Instant::now();
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
        KeyCode::Char('z') => {
            app.flamegraph_view.state.toggle_freeze();
        }
        KeyCode::Char('#') => {
            app.search_selected();
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
        // Other handlers you could add here.
        _ => {
            key_handled = false;
        }
    }
    if key_handled && app.transient_message.is_some() {
        app.clear_transient_message();
    }
    app.add_elapsed("handle_key_events", tic.elapsed());
    Ok(())
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
