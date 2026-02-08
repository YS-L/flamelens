/// Application.
pub mod app;

/// Terminal events handler.
pub mod event;

/// Widget renderer.
pub mod ui;

/// Terminal user interface.
pub mod tui;

/// Event handler.
pub mod handler;

pub mod flame;

pub mod state;

pub mod view;

#[cfg(feature = "python")]
pub mod py_spy;

#[cfg(feature = "python")]
pub mod py_spy_flamegraph;

// Public API for embedding flamelens as a library
use app::{App, AppResult};
use event::{Event, EventHandler};
use flame::FlameGraph;
use handler::handle_key_events;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::{Arc, Mutex};

/// Run interactive flamegraph viewer with static data
///
/// # Arguments
/// * `data` - Collapsed stack data (format: "func1;func2;func3 count")
/// * `title` - Title to display in the viewer
/// * `sorted` - Whether to sort stacks by time spent
///
/// # Example
/// ```no_run
/// let data = "main;foo;bar 100\nmain;foo;baz 50".to_string();
/// flamelens::run_from_collapsed_stacks(data, "my-profile", false)?;
/// ```
pub fn run_from_collapsed_stacks(data: String, title: &str, sorted: bool) -> AppResult<()> {
    let flamegraph = FlameGraph::from_string(data, sorted);
    let mut app = App::with_flamegraph(title, flamegraph);

    run_tui_loop(&mut app)
}

/// Run interactive flamegraph viewer with live updates
///
/// Receives collapsed stack data via mpsc channel and accumulates samples over time.
/// The flamegraph updates as new data arrives.
///
/// # Arguments
/// * `rx` - Channel receiver for collapsed stack data
/// * `title` - Title to display in the viewer
///
/// # Example
/// ```no_run
/// use std::sync::mpsc;
/// let (tx, rx) = mpsc::channel();
///
/// // Spawn thread to send updates
/// std::thread::spawn(move || {
///     tx.send("main;foo 10".to_string()).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     tx.send("main;bar 20".to_string()).unwrap();
/// });
///
/// flamelens::run_from_live_stream(rx, "my-profile [live]")?;
/// ```
pub fn run_from_live_stream(rx: std::sync::mpsc::Receiver<String>, title: &str) -> AppResult<()> {
    // Start with empty flamegraph
    let flamegraph = FlameGraph::from_string(String::new(), true);
    let mut app = App::with_flamegraph(title, flamegraph);

    // Shared state for incoming data
    let next_data = Arc::new(Mutex::new(None::<String>));
    let next_data_clone = next_data.clone();

    // Spawn thread to receive updates
    std::thread::spawn(move || {
        while let Ok(data) = rx.recv() {
            *next_data_clone.lock().unwrap() = Some(data);
        }
    });

    // Store accumulated data
    let mut accumulated_stacks = Vec::new();

    // TUI loop with live updates
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = tui::Tui::new(terminal, events);
    tui.init()?;

    while app.running {
        // Check for new data
        if let Some(new_data) = next_data.lock().unwrap().take() {
            // Accumulate: merge new stacks with existing
            accumulated_stacks = merge_collapsed_stacks(&accumulated_stacks, &new_data);
            let combined = accumulated_stacks.join("\n");

            // Parse and update flamegraph
            let flamegraph = FlameGraph::from_string(combined, true);
            app.flamegraph_view.replace_flamegraph(flamegraph);
        }

        tui.draw(&mut app)?;
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    tui.exit()?;
    Ok(())
}

// Helper: shared TUI loop logic for static mode
fn run_tui_loop(app: &mut App) -> AppResult<()> {
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = tui::Tui::new(terminal, events);
    tui.init()?;

    while app.running {
        tui.draw(app)?;
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    tui.exit()?;
    Ok(())
}

// Helper: Merge collapsed stack format (accumulate counts)
fn merge_collapsed_stacks(existing: &[String], new_data: &str) -> Vec<String> {
    use std::collections::HashMap;

    let mut counts: HashMap<String, u64> = HashMap::new();

    // Parse existing
    for line in existing {
        if let Some((stack, count_str)) = line.rsplit_once(' ') {
            if let Ok(count) = count_str.parse::<u64>() {
                *counts.entry(stack.to_string()).or_insert(0) += count;
            }
        }
    }

    // Add new
    for line in new_data.lines() {
        if let Some((stack, count_str)) = line.rsplit_once(' ') {
            if let Ok(count) = count_str.parse::<u64>() {
                *counts.entry(stack.to_string()).or_insert(0) += count;
            }
        }
    }

    // Reconstruct
    let mut result: Vec<String> = counts
        .into_iter()
        .map(|(stack, count)| format!("{} {}", stack, count))
        .collect();

    result.sort();
    result
}
