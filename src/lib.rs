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
