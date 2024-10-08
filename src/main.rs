use clap::{command, Parser};
use flamelens::app::{App, AppResult};
use flamelens::event::{Event, EventHandler};
use flamelens::flame::FlameGraph;
use flamelens::handler::handle_key_events;
use flamelens::tui::Tui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Profile data filename
    filename: Option<String>,

    /// Whether to sort the stacks by time spent
    #[clap(long, action, value_name = "sorted")]
    sorted: bool,

    /// Print data to stdout on exit. Useful when piping to other tools
    #[clap(long, action, value_name = "echo")]
    echo: bool,

    /// Pid for live flamegraph
    #[cfg(feature = "python")]
    #[clap(long, value_name = "pid")]
    pid: Option<String>,

    /// Additional arguments to pass to "py-spy record" command
    #[cfg(feature = "python")]
    #[clap(long, value_name = "py-spy-args")]
    py_spy_args: Option<String>,

    /// Show debug info
    #[clap(long)]
    debug: bool,
}

fn get_app_from_filename_or_stdin(args: &Args, echo: bool) -> App {
    let (filename, content) = if let Some(filename) = &args.filename {
        (
            filename.as_str(),
            std::fs::read_to_string(filename).expect("Could not read file"),
        )
    } else {
        let mut buf: Vec<u8> = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .expect("Could not read stdin");
        let content = String::from_utf8(buf).expect("Could not parse stdin");
        ("stdin", content)
    };
    if echo {
        println!("{}", content);
    }
    let tic = std::time::Instant::now();
    let flamegraph = FlameGraph::from_string(content, args.sorted);
    let mut app = App::with_flamegraph(filename, flamegraph);
    app.add_elapsed("flamegraph", tic.elapsed());
    app
}

fn main() -> AppResult<()> {
    let args = Args::parse();

    // Create an application.
    cfg_if::cfg_if! {
        if #[cfg(feature = "python")] {
            let mut app = if let Some(_pid) = args.pid {
                App::with_pid(
                    _pid.parse().expect("Could not parse pid"),
                    args.py_spy_args.clone(),
                )
            } else {
                get_app_from_filename_or_stdin(&args, args.echo)
            };
        } else {
            let mut app = get_app_from_filename_or_stdin(&args, args.echo);
        }
    }
    app.debug = args.debug;

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
