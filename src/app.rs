use crate::flame::FlameGraph;
use crate::py_spy::{record_samples, SamplerStatus};
use crate::state::FlameGraphState;
use crate::view::FlameGraphView;
use remoteprocess;
use std::sync::{Arc, Mutex};
use std::{error, thread};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub enum FlameGraphInput {
    File(String),
    Pid(u64, Option<String>),
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// counter
    pub counter: u8,
    /// Flamegraph view
    pub flamegraph_view: FlameGraphView,
    /// Flamegraph input information
    pub flamegraph_input: FlameGraphInput,
    /// Next flamegraph to swap in
    next_flamegraph: Arc<Mutex<Option<FlameGraph>>>,
    sampler_status: Option<Arc<Mutex<SamplerStatus>>>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn with_flamegraph(filename: &str, flamegraph: FlameGraph) -> Self {
        Self {
            running: true,
            counter: 0,
            flamegraph_view: FlameGraphView::new(flamegraph),
            flamegraph_input: FlameGraphInput::File(filename.to_string()),
            next_flamegraph: Arc::new(Mutex::new(None)),
            sampler_status: None,
        }
    }

    pub fn with_pid(pid: u64) -> Self {
        let next_flamegraph: Arc<Mutex<Option<FlameGraph>>> = Arc::new(Mutex::new(None));
        let pyspy_data: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let sampler_status = Arc::new(Mutex::new(SamplerStatus::Running));

        // Thread to poll data from pyspy and construct the next flamegraph
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

        // pyspy live sampler thread
        {
            let pyspy_data = pyspy_data.clone();
            let sampler_status = sampler_status.clone();
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
                record_samples(pid, &config, pyspy_data, sampler_status);
            });
        }

        let flamegraph = FlameGraph::from_string("");
        let process_info = remoteprocess::Process::new(pid as remoteprocess::Pid)
            .and_then(|p| p.cmdline())
            .ok()
            .map(|c| c.join(" "));
        Self {
            running: true,
            counter: 0,
            flamegraph_view: FlameGraphView::new(flamegraph),
            flamegraph_input: FlameGraphInput::Pid(pid, process_info),
            next_flamegraph: next_flamegraph.clone(),
            sampler_status: Some(sampler_status),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        if let Some(fg) = self.next_flamegraph.lock().unwrap().take() {
            self.flamegraph_view.replace_flamegraph(fg);
        }
        if let Some(SamplerStatus::Error(s)) = self
            .sampler_status
            .as_ref()
            .map(|s| s.lock().unwrap().clone())
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

    pub fn sampler_status(&self) -> Option<SamplerStatus> {
        self.sampler_status
            .as_ref()
            .map(|s| s.lock().unwrap().clone())
    }
}
