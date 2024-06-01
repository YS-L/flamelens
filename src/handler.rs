use crate::app::{App, AppResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        // Counter handlers
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
        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
