//!
//! The croncat system daemon.
//!

use croncat_pipeline::{
    try_flat_join, Dispatcher, ProviderSystem, Sequencer,
};
use std::sync::Arc;
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    task::JoinHandle,
};
use tracing::{debug, log::error};

use crate::{
    channels::ShutdownTx,
    config::ChainConfig,
    errors::{eyre, Report},
    logging::info,
    modules::{
        agent::{check_status_loop, Agent},
        factory::{refresh_factory_loop, Factory},
        manager::Manager,
        polling::poll_stream_blocks,
        tasks::{evented_tasks_loop, scheduled_tasks_loop, Tasks},
    },
    tokio,
};

pub mod service;

pub use service::DaemonService;

///
/// Kick off the croncat daemon
///
pub async fn run(
    chain_id: &String,
    shutdown_tx: &ShutdownTx,
    config: &ChainConfig,
    factory: Factory,
    agent: Arc<Agent>,
    manager: Arc<Manager>,
    tasks: Arc<Tasks>,
) -> Result<(), Report> {
    // Get the status of the agent
    let account_id = agent.account_id();
    let account_addr = account_id.clone();
    let status = agent.get_status(account_addr).await?;
    info!("[{}] Agent: {}", chain_id, account_id);
    info!("[{}] Current Status: {:?}", chain_id, status);
    let status = Arc::new(Mutex::new(status));

    // Create a channel for block sources
    let (block_source_tx, block_source_rx) = mpsc::unbounded_channel();

    // Create a provider system for the polling streams.
    let mut provider_system = ProviderSystem::new(block_source_tx, shutdown_tx.clone());

    // For each RPC endpoint, spawn a task to stream blocks from it
    for (provider, data_source) in &config.data_sources() {
        info!(
            "[{}] Starting polling task for {} {}",
            chain_id, provider, data_source.rpc
        );

        provider_system.add_provider_stream(
            provider,
            poll_stream_blocks(data_source.rpc.clone(), config.block_polling_seconds),
        );
    }

    // Provider system monitor updates.
    // let (provider_system_monitor_tx, _provider_system_monitor_rx) = mpsc::channel(100);
    // let provider_system_monitor = ProviderSystemMonitor::new(
    //     provider_system.get_provider_states(),
    //     provider_system_monitor_tx,
    // );

    // Monitor the provider system for updates.
    let provider_system_handle = tokio::spawn(async move { provider_system.produce().await });
    // let _provider_system_monitor_handle =
    //     tokio::spawn(async move { provider_system_monitor.monitor(1000).await });
    // let provider_system_monitor_display_chain_id = chain_id.clone();
    // let _provider_system_monitor_display_handle = tokio::spawn(async move {
    //     let mut provider_system_monitor_rx = _provider_system_monitor_rx;
    //     while let Some(provider_states) = provider_system_monitor_rx.recv().await {
    //         debug!(
    //             "[{}] Provider states: {:#?}",
    //             provider_system_monitor_display_chain_id, provider_states
    //         );
    //     }
    // });

    // Sequence the blocks we receive from the block stream. This is necessary because we may receive
    // blocks from multiple sources, and we need to ensure that we process them in order.
    let (sequencer_tx, sequencer_rx) = mpsc::unbounded_channel();
    let sequencer = Sequencer::new(block_source_rx, sequencer_tx, shutdown_tx.subscribe(), 512)?;
    let sequencer_handle = tokio::task::spawn(async move { sequencer.consume().await });

    // Dispatch blocks to anybody who is listening.
    let (dispatcher_tx, _dispatcher_rx) = broadcast::channel(32);
    let dispatcher = Dispatcher::new(sequencer_rx, dispatcher_tx.clone(), shutdown_tx.subscribe());
    let dispatcher_handle = tokio::task::spawn(async move { dispatcher.fanout().await });

    // Task to show blocks from the block stream
    let mut block_stream_info_rx = dispatcher_tx.subscribe();
    let block_stream_chain_id = chain_id.clone();
    let _block_stream_info_handle = tokio::task::spawn(async move {
        while let Ok(status) = block_stream_info_rx.recv().await {
            debug!(
                "[{}] Processing block (height: {})",
                block_stream_chain_id,
                status.inner.sync_info.latest_block_height,
            );
        }
    });

    // Factory Cache checks
    let factory_cache_check_shutdown_rx = shutdown_tx.subscribe();
    let factory_cache_check_block_stream_rx = dispatcher_tx.subscribe();
    let factory_cache_check_handle = tokio::task::spawn(refresh_factory_loop(
        factory_cache_check_block_stream_rx,
        factory_cache_check_shutdown_rx,
        Arc::new(chain_id.clone()),
        factory,
    ));

    // Account status checks
    let account_status_check_shutdown_rx = shutdown_tx.subscribe();
    let account_status_check_block_stream_rx = dispatcher_tx.subscribe();
    let block_status = status.clone();
    let block_status_accounts_loop = block_status.clone();
    let account_status_check_handle = tokio::task::spawn(check_status_loop(
        account_status_check_block_stream_rx,
        account_status_check_shutdown_rx,
        block_status_accounts_loop,
        Arc::new(chain_id.clone()),
        config.clone(),
        agent.clone(),
        manager.clone(),
    ));

    // Process scheduled tasks based on block stream
    let task_runner_shutdown_rx = shutdown_tx.subscribe();
    let task_runner_block_stream_rx = dispatcher_tx.subscribe();
    let block_status_tasks = block_status.clone();
    let task_runner_handle = tokio::task::spawn(scheduled_tasks_loop(
        task_runner_block_stream_rx,
        task_runner_shutdown_rx,
        block_status_tasks,
        Arc::new(chain_id.clone()),
        agent.clone(),
        manager.clone(),
        tasks.clone(),
    ));

    // Process evented tasks, if they're ready
    let evented_task_runner_shutdown_rx = shutdown_tx.subscribe();
    let evented_task_runner_block_stream_rx = dispatcher_tx.subscribe();
    let block_status_evented_tasks = block_status.clone();
    let evented_task_runner_handle = if let Some(b) = config.include_evented_tasks {
        if b {
            tokio::task::spawn(evented_tasks_loop(
                evented_task_runner_block_stream_rx,
                evented_task_runner_shutdown_rx,
                block_status_evented_tasks,
                Arc::new(chain_id.clone()),
                manager.clone(),
                tasks.clone(),
            ))
        } else {
            tokio::task::spawn(async { Ok(()) })
        }
    } else {
        tokio::task::spawn(async { Ok(()) })
    };

    // TODO: Bring back!
    // // Tasks Cache checks
    // let tasks_cache_check_shutdown_rx = shutdown_tx.subscribe();
    // let tasks_cache_check_block_stream_rx = dispatcher_tx.subscribe();
    // let tasks_cache_check_handle = tokio::task::spawn(refresh_tasks_cache_loop(
    //     tasks_cache_check_block_stream_rx,
    //     tasks_cache_check_shutdown_rx,
    //     Arc::new(chain_id.clone()),
    //     tasks,
    // ));

    // Ctrl-C handler
    let ctrl_c_shutdown_tx = shutdown_tx.clone();
    let ctrl_c_chain_id = chain_id.clone();
    let ctrl_c_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .map_err(|err| eyre!("[{}] Failed to wait for Ctrl-C: {}", ctrl_c_chain_id, err))?;
        ctrl_c_shutdown_tx.send(()).map_err(|err| {
            eyre!(
                "[{}] Failed to send shutdown signal: {}",
                ctrl_c_chain_id,
                err
            )
        })?;
        println!();
        info!("[{}] Shutting down...", ctrl_c_chain_id);

        Ok(())
    });

    // Try to join all the system tasks.
    let system_status = try_flat_join!(
        ctrl_c_handle,
        sequencer_handle,
        dispatcher_handle,
        provider_system_handle,
        factory_cache_check_handle,
        account_status_check_handle,
        task_runner_handle,
        evented_task_runner_handle,
        // tasks_cache_check_handle,
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

pub async fn run_retry(
    chain_id: &String,
    shutdown_tx: &ShutdownTx,
    config: &ChainConfig,
    factory: Factory,
    agent: Arc<Agent>,
    manager: Arc<Manager>,
    tasks: Arc<Tasks>,
) -> Result<(), Report> {
    // TODO: Rethink this retry logic
    // let retry_strategy = FixedInterval::from_millis(5000).take(1200);

    // Retry::spawn(retry_strategy, || async {
    run(
        chain_id,
        shutdown_tx,
        config,
        factory,
        agent,
        manager,
        tasks,
    )
    .await?;
    // .map_err(|err| {
    //     error!("[{}] System crashed: {}", &chain_id, err);
    //     error!("[{}] Retrying...", &chain_id);
    //     err
    // })?;
    // })
    // .await?;

    Ok(())
}
