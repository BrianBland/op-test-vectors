//! Run Op Program Subcommand

use alloy_primitives::hex::ToHexExt;
use clap::{ArgAction, Parser};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use op_test_vectors::faultproof::{ChainDefinition, FaultProofFixture};
use std::process::{Command, ExitStatus};
use std::{env, path::PathBuf};
use tracing::{debug, error, info, trace, warn};

use super::util::RollupConfig;

/// The logging target to use for [tracing].
const TARGET: &str = "run-op-program";

/// CLI arguments for the `run-op-program` subcommand of `opfp`.
#[derive(Parser, Clone, Debug)]
pub struct RunOpProgram {
    /// Path to the op-program binary
    #[clap(short, long, help = "Path to the op-program binary")]
    pub op_program: PathBuf,
    /// Path to the fixture file
    #[clap(short, long, help = "Path to the fixture file")]
    pub fixture: PathBuf,
    /// Optional path to the cannon binary
    #[clap(short, long, help = "Path to the cannon binary")]
    pub cannon: Option<PathBuf>,
    /// Optional cannon state
    #[clap(long, help = "Path to the cannon state")]
    pub cannon_state: Option<PathBuf>,
    /// Optional cannon metadata
    #[clap(long, help = "Path to the cannon metadata")]
    pub cannon_meta: Option<PathBuf>,
    /// Optional cannon debug output file path
    #[clap(long, help = "Path to the cannon debug output file")]
    pub cannon_debug: Option<PathBuf>,
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
}

impl RunOpProgram {
    /// Runs the `run-op-program` subcommand.
    pub async fn run(&self) -> Result<()> {
        let fixture = std::fs::read_to_string(&self.fixture)
            .map_err(|e| eyre!("Failed to read fixture file: {}", e))?;
        let fixture: FaultProofFixture = serde_json::from_str(&fixture)
            .map_err(|e| eyre!("Failed to parse fixture file: {}", e))?;

        let op_program_command =
            OpProgramCommand::new(self.op_program.clone(), self.fixture.clone(), fixture);

        match self.cannon.as_ref() {
            Some(cannon) => {
                let cannon_command = CannonCommand::new(
                    cannon.clone(),
                    self.cannon_state.clone().unwrap(),
                    self.cannon_meta.clone().unwrap(),
                    self.cannon_debug.clone().unwrap(),
                    op_program_command,
                );
                let status = cannon_command.run().await?;
                if !status.success() {
                    error!(target: TARGET, "Failed to execute cannon binary");
                    return Err(eyre!("Failed to execute cannon binary"));
                }
            }
            None => {
                let status = op_program_command.run().await?;
                if !status.success() {
                    error!(target: TARGET, "Failed to execute op-program binary");
                    return Err(eyre!("Failed to execute op-program binary"));
                }
            }
        }

        Ok(())
    }
}

/// The command to run the op-program within cannon.
#[derive(Debug)]
pub struct CannonCommand {
    /// The path to the cannon binary.
    pub cannon: PathBuf,
    /// The path to the cannon state file.
    pub state: PathBuf,
    /// The path to the cannon metadata file.
    pub meta: PathBuf,
    /// The path to the cannon debug output file.
    pub debug: PathBuf,
    /// The op-program command to run within cannon.
    pub op_program: OpProgramCommand,
}

impl CannonCommand {
    pub fn new(
        cannon: PathBuf,
        state: PathBuf,
        meta: PathBuf,
        debug: PathBuf,
        op_program: OpProgramCommand,
    ) -> Self {
        Self {
            cannon,
            state,
            meta,
            debug,
            op_program,
        }
    }

    pub async fn run(&self) -> Result<ExitStatus> {
        let result = Command::new(&self.cannon).args(self.args()).status();

        if let Some(data_dir) = &self.op_program.data_dir {
            std::fs::remove_dir_all(data_dir).unwrap();
        }

        result.map_err(|e| eyre!(e))
    }

    pub fn args(&self) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "--info-at".to_string(),
            "%10000000".to_string(),
            "--proof-at".to_string(),
            "=100000000000".to_string(),
            "--stop-at".to_string(),
            "=200000000000".to_string(),
            "--snapshot-at".to_string(),
            "%10000000000".to_string(),
            "--input".to_string(),
            self.state.to_str().unwrap().to_string(),
            "--meta".to_string(),
            self.meta.to_str().unwrap().to_string(),
            "--debug-info".to_string(),
            self.debug.to_str().unwrap().to_string(),
            "--".to_string(),
            self.op_program.op_program.to_str().unwrap().to_string(),
        ];
        args.extend(self.op_program.args());
        args.push("--server".to_string());
        args
    }
}

/// The command to run the op-program.
#[derive(Debug)]
pub struct OpProgramCommand {
    /// The path to the op-program binary.
    pub op_program: PathBuf,
    pub fixture_path: PathBuf,
    /// The fixture to run the op-program with.
    pub fixture: FaultProofFixture,
    /// The directory to store the input data for the op-program.
    pub data_dir: Option<PathBuf>,
}

impl OpProgramCommand {
    pub fn new(op_program: PathBuf, fixture_path: PathBuf, fixture: FaultProofFixture) -> Self {
        if let ChainDefinition::Unnamed(rollup_config, genesis) = &fixture.inputs.chain_definition {
            let data_dir = env::temp_dir().join("op-program-input");
            if data_dir.exists() {
                std::fs::remove_dir_all(&data_dir).unwrap();
            }
            std::fs::create_dir(&data_dir).unwrap();

            // Write the genesis file to the temp directory.
            let genesis_file = data_dir.join("genesis.json");
            let file = std::fs::File::create(&genesis_file).unwrap();
            serde_json::to_writer_pretty(file, &genesis).unwrap();

            // Write the rollup config to the temp directory.
            let rollup_config_file = data_dir.join("rollup_config.json");
            let file = std::fs::File::create(&rollup_config_file).unwrap();
            let mut cfg: RollupConfig = rollup_config.into();
            cfg.channel_timeout_bedrock = 8;
            serde_json::to_writer_pretty(file, &cfg).unwrap();

            Self {
                op_program,
                fixture_path,
                fixture,
                data_dir: Some(data_dir),
            }
        } else {
            Self {
                op_program,
                fixture_path,
                fixture,
                data_dir: None,
            }
        }
    }

    pub async fn run(&self) -> Result<ExitStatus> {
        let result = Command::new(&self.op_program).args(self.args()).status();

        if let Some(data_dir) = &self.data_dir {
            std::fs::remove_dir_all(data_dir).unwrap();
        }

        result.map_err(|e| eyre!(e))
    }

    pub fn args(&self) -> Vec<String> {
        let mut args = vec![
            "--l1.head".to_string(),
            self.fixture.inputs.l1_head.to_string(),
            "--l2.head".to_string(),
            self.fixture.inputs.l2_head.to_string(),
            "--l2.outputroot".to_string(),
            self.fixture.inputs.l2_output_root.encode_hex_with_prefix(),
            "--l2.blocknumber".to_string(),
            self.fixture.inputs.l2_block_number.to_string(),
            "--l2.claim".to_string(),
            self.fixture.inputs.l2_claim.encode_hex_with_prefix(),
            "--log.format".to_string(),
            "terminal".to_string(),
            "--fixturepath".to_string(),
            self.fixture_path.to_str().unwrap().to_string(),
        ];
        match &self.fixture.inputs.chain_definition {
            ChainDefinition::Named(name) => {
                args.push("--network".to_string());
                args.push(name.to_string());
            }
            ChainDefinition::Unnamed(_, _) => {
                let data_dir = self.data_dir.clone().unwrap();
                args.push("--l2.genesis".to_string());
                args.push(data_dir.join("genesis.json").to_str().unwrap().to_string());
                args.push("--rollup.config".to_string());
                args.push(
                    data_dir
                        .join("rollup_config.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                );
            }
        }
        args
    }
}
