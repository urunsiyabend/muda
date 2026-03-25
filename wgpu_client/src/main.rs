//! Muda Editor - GPU-accelerated desktop client.
//! Thin shell that creates a CoreEditorAdapter and launches ora.

mod adapter;

use std::env;
use std::fs::File;
use std::path::Path;
use log::debug;
use simplelog::{Config, LevelFilter, WriteLogger};

use adapter::CoreEditorAdapter;

fn main() {
    // Initialize logging
    let log_file = File::create("editor.log").expect("Failed to create log file");
    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)
        .expect("Failed to initialize logger");

    debug!("Muda GPU client starting...");

    // Parse args and create editor
    let args: Vec<String> = env::args().collect();
    let editor_app = if args.len() > 1 {
        let path = Path::new(&args[1]);
        if path.is_dir() {
            CoreEditorAdapter::open_directory(&args[1])
                .unwrap_or_else(|_| CoreEditorAdapter::new())
        } else {
            CoreEditorAdapter::open_file(&args[1])
                .unwrap_or_else(|_| CoreEditorAdapter::new())
        }
    } else {
        CoreEditorAdapter::new()
    };

    // Launch ora with the editor adapter — never returns
    ora::run_with_editor(editor_app);
}
