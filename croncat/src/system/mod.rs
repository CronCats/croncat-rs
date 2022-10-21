//!
//! The croncat system daemon.
//!

use std::sync::Arc;
use std::time::Duration;

use cosmrs::bip32::{secp256k1::ecdsa::SigningKey, ExtendedPrivateKey};
use croncat_pipeline::{Dispatcher, Sequencer};
use delegate::delegate;
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    task::JoinHandle,
};
use tokio_retry::{
    strategy::{jitter, FixedInterval},
    Retry,
};
use tracing::log::error;

use crate::{
    channels::{ShutdownRx, ShutdownTx},
    config::{ChainConfig, ChainConfigFile},
    errors::{eyre, Report},
    grpc::GrpcSigner,
    logging::info,
    streams::{agent, polling, tasks, ws},
    tokio,
    utils::flatten_join,
};

pub mod service;

pub use service::DaemonService;

///
/// Block wrapper
///
#[derive(Debug, Clone)]
pub struct Block {
    pub inner: tendermint::Block,
}

#[allow(dead_code)]
impl Block {
    delegate! {
        to self.inner {
            pub fn header(&self) -> &tendermint::block::Header;
            pub fn data(&self) -> &tendermint::abci::transaction::Data;
        }
    }
}

impl From<tendermint::Block> for Block {
    fn from(block: tendermint::Block) -> Self {
        Self { inner: block }
    }
}

impl Ord for Block {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.header().height.cmp(&other.header().height)
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.header().height == other.header().height
    }
}

impl Eq for Block {}

///
/// Kick off the croncat daemon
///
pub async fn run(
    shutdown_tx: ShutdownTx,
    shutdown_rx: ShutdownRx,
    key: ExtendedPrivateKey<SigningKey>,
    config_file: &ChainConfigFile,
    with_rules: bool,
) -> Result<(), Report> {
    // Create a block stream channel
    let (block_stream_tx, block_stream_rx) = mpsc::unbounded_channel();

    // Create a global signer for the chain
    let signer = GrpcSigner::new(
        ChainConfig::from_chain_config_file(config_file, 0),
        key.clone(),
    )
    .await
    .map_err(|err| eyre!("Failed to setup GRPC: {}", err))?;

    // Get the initial agent status
    let initial_status = signer
        .get_agent(signer.account_id().as_ref())
        .await?
        .ok_or(eyre!("Agent must be registered to start the loop"))?
        .status;
    let initial_status = Arc::new(Mutex::new(initial_status));

    // Create shutdown channels.
    let source_shutdown_rx = shutdown_rx.clone();

    // Create a source task for each
    for index in 0..config_file.sources.len() {
        let cfg = ChainConfig::from_chain_config_file(config_file, index as u64);

        // Connect to GRPC  Stream new blocks from the WS RPC subscription
        let wsrpc = signer.wsrpc().to_owned();
        let ws_block_stream_tx = block_stream_tx.clone();

        // Generic retry strategy
        let retry_strategy = FixedInterval::from_millis(3000).map(jitter).take(1200);

        // Handle retries if the websocket task fails.
        //
        // NOTE:
        // This was super tricky, basically we need to pass all non Copyable values
        // in by reference otherwise our closure will be FnOnce and we can't call it.
        //
        // This should be repeated for all the other tasks probably?
        //
        let retry_block_stream_strategy = retry_strategy.clone();
        let retry_block_stream_shutdown_rx = source_shutdown_rx.clone();
        let _retry_block_stream_handle = tokio::task::spawn(async move {
            Retry::spawn(retry_block_stream_strategy, || async {
                // Stream blocks
                ws::stream_blocks_loop(&wsrpc, &ws_block_stream_tx, &retry_block_stream_shutdown_rx)
                    .await
                    .map_err(|err| {
                        error!("Error streaming blocks: {}", err);
                        err
                    })
            })
            .await
        });

        // Set up polling
        let block_polling_shutdown_rx = source_shutdown_rx.clone();
        let rpc_addr = signer.rpc().to_owned();
        let http_block_stream_tx = block_stream_tx.clone();

        // Handle retries if the polling task fails.
        let retry_polling_strategy = retry_strategy.clone();
        let retry_polling_duration_seconds = Duration::from_secs(cfg.polling_duration_secs);
        let _retry_polling_handle = tokio::task::spawn(async move {
            Retry::spawn(retry_polling_strategy, || async {
                // Poll for new blocks
                polling::poll(
                    retry_polling_duration_seconds,
                    &http_block_stream_tx,
                    &block_polling_shutdown_rx,
                    &rpc_addr,
                )
                .await
                .map_err(|err| {
                    error!("Error polling blocks: {}", err);
                    err
                })
            })
            .await
        });
    }

    // Create a sequencer to dedup and sort the blocks with a cache size of 32.
    let (sequencer_tx, sequencer_rx) = mpsc::unbounded_channel();
    let mut sequencer = Sequencer::new(block_stream_rx, sequencer_tx, 128)?;
    let _sequencer_handle = tokio::spawn(async move { sequencer.consume().await });

    // Dispatch the blocks to the indexer.
    let (dispatcher_tx, dispatcher_rx) = broadcast::channel(512);
    let mut dispatcher = Dispatcher::new(sequencer_rx, dispatcher_tx.clone());
    let _dispatcher_handle = tokio::spawn(async move { dispatcher.fanout().await });

    // Account status checks
    let account_status_check_shutdown_rx = source_shutdown_rx.clone();
    let account_status_check_block_stream_rx = dispatcher_tx.subscribe();
    let block_status = initial_status.clone();
    let block_status_accounts_loop = block_status.clone();
    let signer_status = signer.clone();
    let account_status_check_handle = tokio::task::spawn(agent::check_account_status_loop(
        account_status_check_block_stream_rx,
        account_status_check_shutdown_rx,
        block_status_accounts_loop,
        signer_status,
    ));

    // Process blocks coming in from the blockchain
    let task_runner_shutdown_rx = source_shutdown_rx.clone();
    let task_runner_block_stream_rx = dispatcher_tx.subscribe();
    let tasks_signer = signer.clone();
    let block_status_tasks = initial_status.clone();
    let task_runner_handle = tokio::task::spawn(tasks::tasks_loop(
        task_runner_block_stream_rx,
        task_runner_shutdown_rx,
        tasks_signer,
        block_status_tasks,
    ));

    // Check rules if enabled
    let rules_runner_handle = if with_rules {
        tokio::task::spawn(tasks::rules_loop(
            dispatcher_rx,
            source_shutdown_rx,
            signer,
            initial_status.clone(),
        ))
    } else {
        tokio::task::spawn(async { Ok(()) })
    };

    // Handle SIGINT AKA Ctrl-C
    let ctrl_c_shutdown_tx = shutdown_tx.clone();
    let ctrl_c_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .map_err(|err| eyre!("Failed to wait for Ctrl-C: {}", err))?;
        ctrl_c_shutdown_tx
            .broadcast(())
            .await
            .map_err(|err| eyre!("Failed to send shutdown signal: {}", err))?;
        println!();
        info!("Shutting down croncatd...");

        Ok(())
    });

    // Try to join all the system tasks.
    let system_status = tokio::try_join!(
        flatten_join(account_status_check_handle),
        flatten_join(task_runner_handle),
        flatten_join(rules_runner_handle),
        flatten_join(ctrl_c_handle),
    );

    // If any of the tasks failed, we need to propagate the error.
    match system_status {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("croncatd shutdown with error");
            Err(err)
        }
    }
}
