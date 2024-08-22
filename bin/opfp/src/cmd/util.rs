use alloy_primitives::B256;
use alloy_provider::{Provider, ReqwestProvider};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use superchain_registry::BlockID;

/// Represents the response containing the l2 output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputResponse {
    /// The output format version.
    pub version: B256,
    /// The hash of the output.
    pub output_root: B256,
    /// The l2 block reference of this output.
    pub block_ref: L2BlockRef,
    /// The storage root of the message passer contract.
    pub withdrawal_storage_root: B256,
    /// The state root at this block reference.
    pub state_root: B256,
}

/// Represents the reference to an L2 block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L2BlockRef {
    /// The hash of the block.
    pub hash: B256,
    /// The number of the block.
    pub number: u64,
    /// The parent hash of the block.
    pub parent_hash: B256,
    /// The timestamp of the block.
    pub time: u64,
    /// The l1 origin of the block.
    pub l1_origin: BlockID,
    /// The sequence number of the block.
    pub sequence_number: u64,
}

/// Represents the response containing the safe head information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeHeadResponse {
    /// The L1 block reference of the safe head.
    pub l1_block: BlockID,
    /// The L2 block reference of the safe head.
    pub safe_head: BlockID,
}

/// A provider for the rollup node.
#[derive(Debug)]
pub struct RollupProvider {
    /// The inner Ethereum JSON-RPC provider.
    inner: ReqwestProvider,
}

impl RollupProvider {
    /// Creates a new [RollupProvider] with the given alloy provider.
    pub fn new(inner: ReqwestProvider) -> Self {
        Self { inner }
    }

    /// Returns the output at a given block number.
    pub async fn output_at_block(&mut self, block_number: u64) -> Result<OutputResponse> {
        let block_num_hex = format!("0x{:x}", block_number);
        let raw_output = self
            .inner
            .raw_request("optimism_outputAtBlock".into(), (block_num_hex,))
            .await?;
        let output: OutputResponse = serde_json::from_value(raw_output)?;
        Ok(output)
    }

    /// Returns the safe head at an L1 block number.
    pub async fn safe_head_at_block(&mut self, block_number: u64) -> Result<SafeHeadResponse> {
        let block_num_hex = format!("0x{:x}", block_number);
        let raw_resp = self
            .inner
            .raw_request("optimism_safeHeadAtL1Block".into(), (block_num_hex,))
            .await?;
        let resp: SafeHeadResponse = serde_json::from_value(raw_resp)?;
        Ok(resp)
    }

    /// Creates a new [RollupProvider] from the provided [reqwest::Url].
    pub fn new_http(url: reqwest::Url) -> Self {
        // let pb = ProviderBuilder::default().
        let inner = ReqwestProvider::new_http(url);
        Self::new(inner)
    }
}
