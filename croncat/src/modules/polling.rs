//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!
use super::block_pid::BlockPid;
use crate::utils::Block;
use crate::utils::Status;
use async_stream::try_stream;
use color_eyre::{eyre::eyre, Report};
use futures_util::TryStream;
use std::{
    pin::Pin,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tendermint::Time;
use tendermint_rpc::{Client, HttpClient};
use tokio::time::{sleep, timeout};
use tracing::debug;

// The Full block details
type BlockStream =
    Pin<Box<dyn TryStream<Item = Result<Block, Report>, Ok = Block, Error = Report> + Send>>;

// Sync status with block heights
type StatusStream =
    Pin<Box<dyn TryStream<Item = Result<Status, Report>, Ok = Status, Error = Report> + Send>>;

///
/// Stream polled block sync info from the given rpc endpoint.
///
pub fn poll_stream_blocks(http_rpc_host: String, poll_duration_secs: f64) -> StatusStream {
    Box::pin(try_stream! {
        let client = HttpClient::new(http_rpc_host.as_str()).map_err(|source| eyre!("Failed to connect to RPC: {}", source))?;
        let mut block_pid_cache = BlockPid::default();

        // since block heights are ~6secs, don't want to have timeout 30 seconds for failures
        let poll_timeout_duration = Duration::from_secs_f64(poll_duration_secs);
        loop {
            let rpc_request_start = SystemTime::now();
            debug!("rpc_request_start {:?}", rpc_request_start);

            let next_duration = match timeout(poll_timeout_duration, client.status()).await {
                Ok(Ok(status)) => {
                    // For debugging - find out the RPC latency
                    debug!("RPC Latency {:?} {:?}", rpc_request_start.elapsed(), SystemTime::now());

                    let block = status.clone().sync_info;
                    debug!("Polled block {} {}", block.latest_block_height, block.latest_block_time);
                    let block_millis = block.latest_block_time.duration_since(Time::unix_epoch()).unwrap().as_millis();
                    let block_height = block.latest_block_height;

                    // Set the default if this is first known block
                    if block_pid_cache.current.0 == 0 {
                        let previous_block_ht = block_height.clone().value().saturating_sub(1);
                        block_pid_cache.current = (previous_block_ht, block_millis.clone());
                        // Add "previous" block based on our default duration to kick off with a semi-reasonable duration
                        let previous_block_ts = block_millis.saturating_sub(poll_timeout_duration.as_millis());
                        block_pid_cache.height.insert(previous_block_ht, previous_block_ts);
                    }

                    let now = SystemTime::now();
                    let now_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards bruh");
                    let now_millis = now_epoch.as_millis();
                    debug!("DISTANCE FROM LAST BLOCK {:?} {:?} {:?} {:?}", now_millis - block_millis, now_millis, block_millis, block_height);

                    let (next_duration, next_variance) = block_pid_cache.get_next(
                        now_millis,
                        block_height.clone().value(),
                        block_millis,
                    );

                    debug!(
                        "Estimated next block time (duration: {:?}) variance {:?}",
                        next_duration, next_variance
                    );

                    // yield status.into();
                    let stat = Status {
                        inner: status.clone(),
                    };
                    yield stat.into();

                    next_duration
                }
                Ok(Err(err)) => {
                    debug!("Failed to get latest block: {}", err);
                    poll_timeout_duration
                }
                Err(err) => {
                    debug!("Timed out getting latest block: {}", err);
                    poll_timeout_duration
                }
            };

            // Wait
            sleep(next_duration).await;
        }
    })
}

///
/// Stream polled blocks from the given rpc endpoint.
/// Streams the entire data of a given block
///
pub fn poll_stream_blocks_detailed(http_rpc_host: String, poll_duration_secs: f64) -> BlockStream {
    Box::pin(try_stream! {
        let client = HttpClient::new(http_rpc_host.as_str()).map_err(|source| eyre!("Failed to connect to RPC: {}", source))?;
        let mut block_pid_cache = BlockPid::default();

        // since block heights are ~6secs, don't want to have timeout 30 seconds for failures
        let poll_timeout_duration = Duration::from_secs_f64(poll_duration_secs);
        loop {
            let rpc_request_start = SystemTime::now();
            debug!("rpc_request_start {:?}", rpc_request_start);

            let next_duration = match timeout(poll_timeout_duration, client.latest_block()).await {
                Ok(Ok(block)) => {
                    // For debugging - find out the RPC latency
                    debug!("RPC Latency {:?} {:?}", rpc_request_start.elapsed(), SystemTime::now());

                    let block = block.block;
                    debug!("[{}] Polled block {}", block.header().chain_id, block.header().height);
                    let block_millis = block.header().time.duration_since(Time::unix_epoch()).unwrap().as_millis();
                    let block_height = block.header().height;

                    // Set the default if this is first known block
                    if block_pid_cache.current.0 == 0 {
                        let previous_block_ht = block_height.clone().value().saturating_sub(1);
                        block_pid_cache.current = (previous_block_ht, block_millis.clone());
                        // Add "previous" block based on our default duration to kick off with a semi-reasonable duration
                        let previous_block_ts = block_millis.saturating_sub(poll_timeout_duration.as_millis());
                        block_pid_cache.height.insert(previous_block_ht, previous_block_ts);
                    }

                    let now = SystemTime::now();
                    let now_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards bruh");
                    let now_millis = now_epoch.as_millis();
                    let (next_duration, next_variance) = block_pid_cache.get_next(
                        now_millis,
                        block_height.clone().value(),
                        block_millis,
                    );

                    debug!(
                        "Estimated next block time (duration: {:?}) variance {:?}",
                        next_duration, next_variance
                    );

                    yield block.into();

                    next_duration
                }
                Ok(Err(err)) => {
                    debug!("Failed to get latest block: {}", err);
                    poll_timeout_duration
                }
                Err(err) => {
                    debug!("Timed out getting latest block: {}", err);
                    poll_timeout_duration
                }
            };

            // Wait
            sleep(next_duration).await;
        }
    })
}
