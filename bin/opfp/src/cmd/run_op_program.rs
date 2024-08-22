//! Run Op Program Subcommand

use alloy_primitives::{
    b256,
    hex::{self, ToHexExt},
    B256, U256,
};
use alloy_provider::{Provider, ProviderBuilder, ReqwestProvider};
use clap::{ArgAction, Parser};
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use hashbrown::HashMap;
use kona_derive::{
    online::*,
    types::{L2BlockInfo, StageError},
};
use op_test_vectors::faultproof::{FaultProofFixture, FaultProofInputs};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    env,
    io::{stderr, stdout},
    path::PathBuf,
};
use std::{
    ffi::OsStr,
    process::{Command, CommandArgs, ExitStatus},
    sync::Arc,
};
use superchain_registry::{BlockID, CHAINS, OPCHAINS, ROLLUP_CONFIGS};
use tracing::{debug, error, info, trace, warn};

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

        // let temp_input_dir = env::temp_dir().join("op-program-input");
        // if temp_input_dir.exists() {
        //     std::fs::remove_dir_all(&temp_input_dir).unwrap();
        //     std::fs::create_dir(&temp_input_dir)?;
        // } else {
        //     std::fs::create_dir(&temp_input_dir)?;
        // }
        // for (key, value) in &fixture.witness_data {
        //     let file = temp_input_dir.join(format!("{}.txt", key.encode_hex_with_prefix()));
        //     std::fs::write(file, value).map_err(|e| eyre!("Failed to write input file: {}", e))?;
        // }

        let op_program_command = OpProgramCommand::new(self.op_program.clone(), fixture);

        match self.cannon.as_ref() {
            Some(cannon) => {
                let status = Command::new(cannon)
                    .arg("run")
                    .arg("--info-at")
                    .arg("%10000000")
                    .arg("--proof-at")
                    .arg("=100000000000")
                    .arg("--stop-at")
                    .arg("=200000000000")
                    .arg("--snapshot-at")
                    .arg("%10000000000")
                    .arg("--input")
                    .arg(self.cannon_state.as_ref().unwrap())
                    .arg("--meta")
                    .arg(self.cannon_meta.as_ref().unwrap())
                    .arg("--debug-info")
                    .arg(self.cannon_debug.as_ref().unwrap())
                    .arg("--")
                    .arg(op_program_command.op_program.clone())
                    .args(op_program_command.args())
                    .arg("--server")
                    .stdout(stdout())
                    .stderr(stderr())
                    .status()
                    .map_err(|e| eyre!(e))?;
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

pub struct OpProgramCommand {
    pub op_program: PathBuf,
    pub fixture: FaultProofFixture,
    pub data_dir: PathBuf,
}

impl OpProgramCommand {
    pub fn new(op_program: PathBuf, fixture: FaultProofFixture) -> Self {
        let data_dir = env::temp_dir().join("op-program-input");
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir).unwrap();
            std::fs::create_dir(&data_dir).unwrap();
        } else {
            std::fs::create_dir(&data_dir).unwrap();
        }
        for (key, value) in &fixture.witness_data {
            let file = data_dir.join(format!("{}.txt", key.encode_hex_with_prefix()));
            std::fs::write(file, value).unwrap();
        }
        Self {
            op_program,
            fixture,
            data_dir,
        }
    }

    pub async fn run(&self) -> Result<ExitStatus> {
        let args = self.args();
        Command::new(&self.op_program)
            .args(args)
            .stdout(stdout())
            .stderr(stderr())
            .status()
            .map_err(|e| eyre!(e))
    }

    pub fn args(&self) -> Vec<String> {
        vec![
            "--network".to_string(),
            "base-sepolia".to_string(),
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
            "--datadir".to_string(),
            self.data_dir.to_str().unwrap().to_string(),
        ]
    }
}
