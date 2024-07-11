use crate::opt8n::Opt8n;
use anvil::cmd::NodeArgs;
use clap::{Command, CommandFactory, Parser, Subcommand};
use color_eyre::eyre;
use forge_script::ScriptArgs;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::trace;

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub node_args: NodeArgs,
}

#[derive(Subcommand, Clone, Debug)]
#[clap(rename_all = "kebab_case", infer_subcommands = true)]
pub enum Commands {
    /// Uses a forge script to generate a test vector
    #[command(visible_alias = "s")]
    Script {
        #[command(flatten)]
        script_args: ScriptArgs,
    },

    /// Starts a REPL for running forge, anvil, and cast commands
    #[command(visible_alias = "r")]
    Repl {},
}

impl Cli {
    pub async fn run(self) -> eyre::Result<()> {
        let node_config = self.node_args.into_node_config();
        let mut opt8n = Opt8n::new(node_config).await;

        match &self.command {
            Commands::Script { script_args } => {
                println!("Running script: {}", script_args.path);
                Ok(())
            }
            Commands::Repl { .. } => {
                println!("Starting REPL");
                opt8n.listen().await;
                Ok(())
            }
        }
    }

    // Modify the cli with sensible defaults
    pub fn default_command() -> Command {
        Cli::command_for_update().mut_args(|mut arg| {
            match arg.get_id().as_str() {
                "optimism" => {
                    trace!("Setting node-args as optional");
                    arg = arg.default_value("true");
                }
                _ => {}
            }
            arg
        })
    }
}

#[derive(Parser, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[clap(rename_all = "kebab_case", infer_subcommands = true, multicall = true)]
pub enum Opt8nCommand {
    #[command(visible_alias = "a")]
    Anvil {
        #[arg(index = 1, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(visible_alias = "c")]
    Cast {
        #[arg(index = 1, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(visible_alias = "e")]
    Exit,
}
