//! clap CLI surface.
mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "nixpresence",
    version,
    about = "NixOS VRChat chatbox + Discord presence"
)]
pub struct Cli {
    /// Increase log verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run the daemon in the foreground (same as daemon)
    Run {
        /// Also print composed lines to stdout
        #[arg(long)]
        stdout: bool,
    },
    /// Run as a long-lived daemon
    Daemon {
        #[arg(long)]
        stdout: bool,
    },
    /// Status helpers
    Status {
        #[command(subcommand)]
        action: StatusCmd,
    },
    /// Preview one composed chatbox line
    Preview {
        /// Profile override
        #[arg(long)]
        profile: Option<String>,
        /// Repeat every tick (Ctrl-C to stop)
        #[arg(long)]
        watch: bool,
    },
    /// Environment diagnostics
    Doctor {
        #[command(subcommand)]
        extra: Option<DoctorCmd>,
    },
    /// Config helpers
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// Module listing / toggles (config edit hints)
    Modules {
        #[command(subcommand)]
        action: ModulesCmd,
    },
    /// Enable a module in config
    Enable { module: String },
    /// Disable a module in config
    Disable { module: String },
    /// Privacy audit of current composition
    Privacy {
        #[command(subcommand)]
        action: PrivacyCmd,
    },
    /// Minimal TUI overview (first-pass)
    Tui,
    /// Send test payloads
    Test {
        #[command(subcommand)]
        target: TestCmd,
    },
    /// Talk to the running daemon over IPC
    Ipc {
        /// Raw line to send (e.g. "status hello")
        message: String,
    },
    /// VRChat helpers
    Vrchat {
        #[command(subcommand)]
        action: VrchatCmd,
    },
    /// Discord helpers
    Discord {
        #[command(subcommand)]
        action: DiscordCmd,
    },
    /// Music / MPRIS helpers
    Music {
        #[command(subcommand)]
        action: MusicCmd,
    },
}

#[derive(Debug, Subcommand)]
pub enum StatusCmd {
    /// Set custom status (IPC if daemon running, else writes override file)
    Set { message: Vec<String> },
    /// Clear custom status
    Clear,
    /// Show current composed status once
    Show,
}

#[derive(Debug, Subcommand)]
pub enum DoctorCmd {
    /// Extra VRChat / Proton checks
    Vrchat,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Write default config.toml
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Print config path
    Path,
    /// Validate config
    Validate {
        #[arg(long)]
        file: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ModulesCmd {
    List,
    Enable { module: String },
    Disable { module: String },
}

#[derive(Debug, Subcommand)]
pub enum PrivacyCmd {
    Audit,
}

#[derive(Debug, Subcommand)]
pub enum TestCmd {
    /// Send a harmless OSC message to host:port (default 127.0.0.1:9000)
    Osc {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 9000)]
        port: u16,
        #[arg(long, default_value = "nixpresence osc test")]
        message: String,
    },
    /// Try Discord IPC connect (requires NIXPRESENCE_DISCORD_APP_ID or config)
    Discord,
    /// Report Kopuz socket / MPRIS
    Kopuz,
}

#[derive(Debug, Subcommand)]
pub enum VrchatCmd {
    /// Detect VRChat process
    Detect,
    /// Force one OSC send (sets notify=false)
    Send { message: Vec<String> },
}

#[derive(Debug, Subcommand)]
pub enum DiscordCmd {
    /// List discord-ipc sockets
    Sockets,
    /// Show music coexistence mode
    Music,
}

#[derive(Debug, Subcommand)]
pub enum MusicCmd {
    /// Show now-playing via MPRIS
    Now,
}

pub fn dispatch(cli: Cli) -> Result<()> {
    commands::run(cli)
}
