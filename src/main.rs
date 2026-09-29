//! nixpresence — NixOS-friendly VRChat chatbox + Discord presence daemon.
mod cli;
mod compositor;
mod config;
mod daemon;
mod diagnostics;
mod ipc;
mod outputs;
mod providers;
mod util;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    init_tracing(cli.verbose);
    cli::dispatch(cli)
}

fn init_tracing(verbose: u8) {
    let default = match verbose {
        0 => "nixpresence=info",
        1 => "nixpresence=debug",
        _ => "nixpresence=trace",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
