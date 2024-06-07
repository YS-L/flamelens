// Part of this file is taken from the py-spy project main.rs
// https://github.com/benfred/py-spy/blob/master/src/flamegraph.rs
// licensed under the MIT License:
/*
The MIT License (MIT)

Copyright (c) 2018-2019 Ben Frederickson

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
use crate::py_spy_flamegraph::Flamegraph as PySpyFlamegraph;
use anyhow::Error;
use py_spy::config::RecordDuration;
use py_spy::sampler;
use py_spy::Config;
use py_spy::Frame;
use remoteprocess;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn record_samples(
    pid: remoteprocess::Pid,
    config: &Config,
    output_data: Arc<Mutex<Option<String>>>,
) -> Result<(), Error> {
    let mut output = PySpyFlamegraph::new(config.show_line_numbers);

    let sampler = sampler::Sampler::new(pid, config)?;

    let max_intervals = match &config.duration {
        RecordDuration::Unlimited => None,
        RecordDuration::Seconds(sec) => Some(sec * config.sampling_rate),
    };

    let mut _errors = 0;
    let mut intervals = 0;
    let mut _samples = 0;

    let mut last_late_message = std::time::Instant::now();
    let mut last_data_dump: Option<Instant> = None;

    for mut sample in sampler {
        if let Some(delay) = sample.late {
            if delay > Duration::from_secs(1) {
                if config.hide_progress {
                    // display a message if we're late, but don't spam the log
                    let now = std::time::Instant::now();
                    if now - last_late_message > Duration::from_secs(1) {
                        last_late_message = now;
                        println!("{:.2?} behind in sampling, results may be inaccurate. Try reducing the sampling rate", delay)
                    }
                } else {
                    println!("{:.2?} behind in sampling, results may be inaccurate. Try reducing the sampling rate.", delay);
                }
            }
        }

        intervals += 1;
        if let Some(max_intervals) = max_intervals {
            if intervals >= max_intervals {
                break;
            }
        }

        for trace in sample.traces.iter_mut() {
            if !(config.include_idle || trace.active) {
                continue;
            }

            if config.gil_only && !trace.owns_gil {
                continue;
            }

            if config.include_thread_ids {
                let threadid = trace.format_threadid();
                let thread_fmt = if let Some(thread_name) = &trace.thread_name {
                    format!("thread ({}): {}", threadid, thread_name)
                } else {
                    format!("thread ({})", threadid)
                };
                trace.frames.push(Frame {
                    name: thread_fmt,
                    filename: String::from(""),
                    module: None,
                    short_filename: None,
                    line: 0,
                    locals: None,
                    is_entry: true,
                });
            }

            if let Some(process_info) = trace.process_info.as_ref() {
                trace.frames.push(process_info.to_frame());
                let mut parent = process_info.parent.as_ref();
                while parent.is_some() {
                    if let Some(process_info) = parent {
                        trace.frames.push(process_info.to_frame());
                        parent = process_info.parent.as_ref();
                    }
                }
            }

            _samples += 1;
            output.increment(trace)?;
        }

        if let Some(sampling_errors) = sample.sampling_errors {
            for (_pid, _e) in sampling_errors {
                _errors += 1;
            }
        }

        let should_dump = match last_data_dump {
            Some(last_data_dump) => {
                let elapsed = Instant::now() - last_data_dump;
                elapsed.as_millis() >= 250
            }
            None => true,
        };
        if should_dump {
            last_data_dump = Some(Instant::now());
            let data = output.get_data();
            // let mut file = std::fs::File::create("data.txt")?;
            // std::io::Write::write_all(&mut file, data.as_bytes())?;
            output_data.lock().unwrap().replace(data);
        }
    }

    Ok(())
}
