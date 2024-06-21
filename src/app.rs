use crate::flame::FlameGraph;
use crate::py_spy::{record_samples, ProfilerOutput, SamplerState, SamplerStatus};
use crate::state::FlameGraphState;
use crate::view::FlameGraphView;
use remoteprocess;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{error, thread};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub enum FlameGraphInput {
    File(String),
    Pid(u64, Option<String>),
}

#[derive(Debug)]
pub struct ParsedFlameGraph {
    pub flamegraph: FlameGraph,
    pub elapsed: Duration,
}

#[derive(Debug)]
pub struct InputBuffer {
    pub buffer: tui_input::Input,
    pub cursor: Option<(u16, u16)>,
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Flamegraph view
    pub flamegraph_view: FlameGraphView,
    /// Flamegraph input information
    pub flamegraph_input: FlameGraphInput,
    /// User input buffer
    pub input_buffer: Option<InputBuffer>,
    /// Timing information for debugging
    pub elapsed: HashMap<String, Duration>,
    /// State of the live sampler
    /// Debug mode
    pub debug: bool,
    /// Next flamegraph to swap in
    next_flamegraph: Arc<Mutex<Option<ParsedFlameGraph>>>,
    sampler_state: Option<Arc<Mutex<SamplerState>>>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn with_flamegraph(filename: &str, flamegraph: FlameGraph) -> Self {
        Self {
            running: true,
            flamegraph_view: FlameGraphView::new(flamegraph),
            flamegraph_input: FlameGraphInput::File(filename.to_string()),
            input_buffer: None,
            elapsed: HashMap::new(),
            debug: false,
            next_flamegraph: Arc::new(Mutex::new(None)),
            sampler_state: None,
        }
    }

    pub fn with_pid(pid: u64) -> Self {
        let next_flamegraph: Arc<Mutex<Option<ParsedFlameGraph>>> = Arc::new(Mutex::new(None));
        let pyspy_data: Arc<Mutex<Option<ProfilerOutput>>> = Arc::new(Mutex::new(None));
        let sampler_state = Arc::new(Mutex::new(SamplerState::default()));

        // Thread to poll data from pyspy and construct the next flamegraph
        {
            let next_flamegraph = next_flamegraph.clone();
            let pyspy_data = pyspy_data.clone();
            let _handle = thread::spawn(move || loop {
                if let Some(output) = pyspy_data.lock().unwrap().take() {
                    let tic = std::time::Instant::now();
                    let flamegraph = FlameGraph::from_string(output.data);
                    let parsed = ParsedFlameGraph {
                        flamegraph,
                        elapsed: tic.elapsed(),
                    };
                    *next_flamegraph.lock().unwrap() = Some(parsed);
                }
                thread::sleep(std::time::Duration::from_millis(250));
            });
        }

        // pyspy live sampler thread
        {
            let pyspy_data = pyspy_data.clone();
            let sampler_state = sampler_state.clone();
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
                record_samples(pid, &config, pyspy_data, sampler_state);
            });
        }

        let flamegraph = FlameGraph::from_string("".to_string());
        let process_info = remoteprocess::Process::new(pid as remoteprocess::Pid)
            .and_then(|p| p.cmdline())
            .ok()
            .map(|c| c.join(" "));
        Self {
            running: true,
            flamegraph_view: FlameGraphView::new(flamegraph),
            flamegraph_input: FlameGraphInput::Pid(pid, process_info),
            next_flamegraph: next_flamegraph.clone(),
            input_buffer: None,
            elapsed: HashMap::new(),
            debug: false,
            sampler_state: Some(sampler_state),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        // Replace flamegraph
        if !self.flamegraph_view.state.freeze {
            if let Some(parsed) = self.next_flamegraph.lock().unwrap().take() {
                self.elapsed
                    .insert("flamegraph".to_string(), parsed.elapsed);
                let tic = std::time::Instant::now();
                self.flamegraph_view.replace_flamegraph(parsed.flamegraph);
                self.elapsed
                    .insert("replacement".to_string(), tic.elapsed());
            }
        }

        // Exit if fatal error in sampler
        if let Some(SamplerStatus::Error(s)) = self
            .sampler_state
            .as_ref()
            .map(|s| s.lock().unwrap().status.clone())
        {
            panic!("py-spy sampler exited with error: {}\n\nYou likely need to rerun this program with sudo.", s);
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

    pub fn sampler_state(&self) -> Option<SamplerState> {
        self.sampler_state
            .as_ref()
            .map(|s| s.lock().unwrap().clone())
    }

    pub fn add_elapsed(&mut self, name: &str, elapsed: Duration) {
        self.elapsed.insert(name.to_string(), elapsed);
    }
}
