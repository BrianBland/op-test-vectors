//! Module for the CLI.

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use tracing::Level;

// pub mod blobs;
// pub mod fixtures;
// pub mod from_l1;
// pub mod from_l2;
// pub mod info;
// pub mod util;
// pub use fixtures::build_fixture_blocks;
pub mod from_op_program;

/// Main CLI
#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommands for the CLI
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for the CLI
#[derive(Parser, Clone, Debug)]
pub enum Commands {
    /// Creates the fault proof fixture from the op-program implementation.
    FromOpProgram(from_op_program::FromOpProgram),
}

impl Cli {
    /// Returns the verbosity level for the CLI
    pub fn v(&self) -> u8 {
        match &self.command {
            Commands::FromOpProgram(cmd) => cmd.v,
        }
    }

    /// Initializes telemtry for the application.
    pub fn init_telemetry(self) -> Result<Self> {
        color_eyre::install()?;
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(match self.v() {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                _ => Level::TRACE,
            })
            .finish();
        tracing::subscriber::set_global_default(subscriber).map_err(|e| eyre!(e))?;
        Ok(self)
    }

    /// Parse the CLI arguments and run the command
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::FromOpProgram(cmd) => cmd.run().await,
        }
    }
}
