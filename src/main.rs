use flamelens::app::{App, AppResult};
use flamelens::event::{Event, EventHandler};
use flamelens::flame::FlameGraph;
use flamelens::handler::handle_key_events;
use flamelens::tui::Tui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

fn main() -> AppResult<()> {
    // flamelens::flame::run();
    main_tui()
}

fn main_tui() -> AppResult<()> {
    // Create an application.
    let filename = std::env::args().nth(1).expect("No filename given");
    let content = std::fs::read_to_string(filename).expect("Could not read file");
    let flamegraph = FlameGraph::from_string(&content);
    let mut app = App::new(flamegraph);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
