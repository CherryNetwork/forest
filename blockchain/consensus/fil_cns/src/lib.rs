// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT
use anyhow::anyhow;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use thiserror::Error;

use forest_beacon::{Beacon, BeaconSchedule};
use forest_blocks::{Block, Tipset};
use forest_chain::Weight;
use forest_chain::{Error as ChainStoreError, Scale};
use forest_chain_sync::Consensus;
use forest_db::Store;
use forest_state_manager::Error as StateManagerError;
use forest_state_manager::StateManager;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::Error as ForestEncodingError;
use nonempty::NonEmpty;

mod metrics;
mod validation;
mod weight;

// Shim to work with daemon.rs
pub mod composition;

#[derive(Debug, Error)]
pub enum FilecoinConsensusError {
    #[error("Block must have an election proof included in tipset")]
    BlockWithoutElectionProof,
    #[error("Block without ticket")]
    BlockWithoutTicket,
    #[error("Block had the wrong timestamp: {0} != {1}")]
    UnequalBlockTimestamps(u64, u64),
    #[error("Tipset without ticket to verify")]
    TipsetWithoutTicket,
    #[error("Block is not claiming to be a winner")]
    NotClaimingWin,
    #[error("Block miner was slashed or is invalid")]
    InvalidOrSlashedMiner,
    #[error("Miner power not available for miner address")]
    MinerPowerNotAvailable,
    #[error("Miner claimed wrong number of wins: miner = {0}, computed = {1}")]
    MinerWinClaimsIncorrect(i64, i64),
    #[error("Drawing chain randomness failed: {0}")]
    DrawingChainRandomness(String),
    #[error("Miner isn't elligible to mine")]
    MinerNotEligibleToMine,
    #[error("Querying miner power failed: {0}")]
    MinerPowerUnavailable(String),
    #[error("Power actor not found")]
    PowerActorUnavailable,
    #[error("Verifying VRF failed: {0}")]
    VrfValidation(String),
    #[error("Failed to validate blocks random beacon values: {0}")]
    BeaconValidation(String),
    #[error("Failed to verify winning PoSt: {0}")]
    WinningPoStValidation(String),
    #[error("[INSECURE-POST-VALIDATION] {0}")]
    InsecurePostValidation(String),
    #[error("Chain store error: {0}")]
    ChainStore(#[from] ChainStoreError),
    #[error("StateManager error: {0}")]
    StateManager(#[from] StateManagerError),
    #[error("Encoding error: {0}")]
    ForestEncoding(#[from] ForestEncodingError),
}

pub struct FilecoinConsensus<B> {
    /// `Drand` randomness beacon
    ///
    /// NOTE: The `StateManager` makes available a beacon as well,
    /// but it potentially has a different type.
    /// Not sure where this is utilized.
    beacon: Arc<BeaconSchedule<B>>,
}

impl<B> FilecoinConsensus<B> {
    pub fn new(beacon: Arc<BeaconSchedule<B>>) -> Self {
        Self { beacon }
    }
}

impl<B> Debug for FilecoinConsensus<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilecoinConsensus")
            .field("beacon", &self.beacon.0.len())
            .finish()
    }
}

impl<B> Scale for FilecoinConsensus<B> {
    fn weight<DB>(db: &DB, ts: &Tipset) -> Result<Weight, anyhow::Error>
    where
        DB: Blockstore,
    {
        weight::weight(db, ts).map_err(|s| anyhow!(s))
    }
}

#[async_trait]
impl<B> Consensus for FilecoinConsensus<B>
where
    B: Beacon + Unpin,
{
    type Error = FilecoinConsensusError;

    async fn validate_block<DB>(
        &self,
        state_manager: Arc<StateManager<DB>>,
        block: Arc<Block>,
    ) -> Result<(), NonEmpty<Self::Error>>
    where
        DB: Blockstore + Store + Clone + Sync + Send + 'static,
    {
        validation::validate_block::<_, _>(state_manager, self.beacon.clone(), block).await
    }
}
