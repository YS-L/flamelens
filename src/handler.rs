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
            app.to_next_sibling();
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.to_previous_sibling();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.to_child_stack();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.to_parent_stack();
        }
        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
