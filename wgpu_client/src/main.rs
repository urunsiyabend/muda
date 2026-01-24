//! Muda Editor - GPU-accelerated desktop client.
//!
//! This binary provides a native desktop experience using:
//! - wgpu for GPU rendering
//! - winit for windowing and input
//! - glyphon for text rendering

mod app;
mod components;
mod input;
mod renderer;
mod theme;

use std::env;
use std::fs::File;
use std::path::Path;

use log::debug;
use simplelog::{Config, LevelFilter, WriteLogger};
use winit::event_loop::EventLoop;

use app::WgpuApp;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    let log_file = File::create("editor.log")?;
    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)?;

    debug!("Muda GPU client starting...");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let app = if args.len() > 1 {
        let path = Path::new(&args[1]);

        if path.is_dir() {
            debug!("Opening directory: {}", &args[1]);
            WgpuApp::open_directory(&args[1])?
        } else {
            debug!("Opening file: {}", &args[1]);
            WgpuApp::open_file(&args[1]).unwrap_or_else(|e| {
                debug!("Could not open file: {}, creating new document", e);
                WgpuApp::new()
            })
        }
    } else {
        WgpuApp::new()
    };

    // Create event loop and run
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut { app })?;

    debug!("Muda GPU client shutting down...");
    Ok(())
}
