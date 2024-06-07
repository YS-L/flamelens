use crate::flame::FlameGraph;
use crate::py_spy::record_samples;
use crate::state::FlameGraphState;
use crate::view::FlameGraphView;
use remoteprocess;
use std::sync::{Arc, Mutex};
use std::{error, thread};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// counter
    pub counter: u8,
    /// Flamegraph view
    pub flamegraph_view: FlameGraphView,
    /// Next flamegraph to swap in
    next_flamegraph: Arc<Mutex<Option<FlameGraph>>>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn with_flamegraph(flamegraph: FlameGraph) -> Self {
        Self {
            running: true,
            counter: 0,
            flamegraph_view: FlameGraphView::new(flamegraph),
            next_flamegraph: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_pid(pid: u64) -> Self {
        let next_flamegraph: Arc<Mutex<Option<FlameGraph>>> = Arc::new(Mutex::new(None));
        let pyspy_data: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        {
            let next_flamegraph = next_flamegraph.clone();
            let pyspy_data = pyspy_data.clone();
            let _handle = thread::spawn(move || loop {
                if let Some(data) = pyspy_data.lock().unwrap().take() {
                    let flamegraph = FlameGraph::from_string(&data);
                    *next_flamegraph.lock().unwrap() = Some(flamegraph);
                }
                thread::sleep(std::time::Duration::from_millis(250));
            });
        }

        {
            let pyspy_data = pyspy_data.clone();
            let _handle = thread::spawn(move || {
                // Note: mimic a record command's invocation vs simply getting default Config as
                // from_args does a lot of heavy lifting
                let args = vec![
                    "py-spy".to_owned(),
                    "record".to_string(),
                    "--pid".to_string(),
                    format!("{}", pid),
                    "--format".to_string(),
                    "raw".to_string(),
                ];
                let config = py_spy::Config::from_args(&args).unwrap();
                let pid = pid as remoteprocess::Pid;
                record_samples(pid, &config, pyspy_data).unwrap();
            });
        }

        let flamegraph = FlameGraph::from_string("");
        Self {
            running: true,
            counter: 0,
            flamegraph_view: FlameGraphView::new(flamegraph),
            next_flamegraph: next_flamegraph.clone(),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        if let Some(fg) = self.next_flamegraph.lock().unwrap().take() {
            self.flamegraph_view.set_flamegraph(fg);
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn flamegraph(&self) -> &FlameGraph {
        &self.flamegraph_view.flamegraph
    }

    pub fn flamegraph_state(&self) -> &FlameGraphState {
        &self.flamegraph_view.state
    }
}
