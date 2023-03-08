//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!
use crate::utils::Block;
use async_stream::try_stream;
use color_eyre::{eyre::eyre, Report};
use futures_util::TryStream;
use tendermint::{Time, block::Height, node::Info};
use std::{pin::Pin, time::{Duration, SystemTime, UNIX_EPOCH}};
use tendermint_rpc::{Client, HttpClient, endpoint::status};
use tokio::{
    time::{sleep, timeout},
};
use tracing::{debug};
use super::block_pid::BlockPid;

type BlockStream =
    Pin<Box<dyn TryStream<Item = Result<Block, Report>, Ok = Block, Error = Report> + Send>>;

// TODO: Migrate to
type StatusStream =
    Pin<Box<dyn TryStream<Item = Result<Block, Report>, Ok = Block, Error = Report> + Send>>;

///
/// Stream polled blocks from the given rpc endpoint.
///
pub fn poll_stream_blocks(http_rpc_host: String, poll_duration_secs: f64) -> BlockStream {
    Box::pin(try_stream! {
        let client = HttpClient::new(http_rpc_host.as_str()).map_err(|source| eyre!("Failed to connect to RPC: {}", source))?;
        let mut block_pid_cache = BlockPid::default();
        let mut previous_block: Height = Height::default();

        // since block heights are ~6secs, don't want to have timeout 30 seconds for failures
        let poll_timeout_duration = Duration::from_secs_f64(poll_duration_secs);
        // let poll_timeout_duration = Duration::from_secs_f64(10.0);
        loop {
            let mut block_height: Height = Height::default();
            let mut next_duration = Duration::from_secs_f64(poll_duration_secs);
            let mut next_variance: u64 = 14;
            let rpc_request_start = SystemTime::now();
            println!("rpc_request_start {:?}", rpc_request_start);
            // match timeout(poll_timeout_duration, client.latest_block()).await {
            match timeout(poll_timeout_duration, client.latest_block()).await {
                Ok(Ok(block)) => {
                    // For debugging - find out the RPC latency
                    println!("RPC Latency {:?} {:?}", rpc_request_start.elapsed(), SystemTime::now());

                    let block = block.block;
                    debug!("[{}] Polled block {}", block.header().chain_id, block.header().height);
                    println!("[{}] Polled block {}", block.header().chain_id, block.header().height);
                    let now = SystemTime::now();
                    let now_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards bruh");
                    let now_millis = now_epoch.as_millis();
                    let block_millis = block.header().time.duration_since(Time::unix_epoch()).unwrap().as_millis();
                    block_height = block.header().height;

                    // Set the default if this is first known block
                    if block_pid_cache.current.0 == 0 {
                        let previous_block_ht = block_height.clone().value().saturating_sub(1);
                        block_pid_cache.current = (previous_block_ht, block_millis.clone());
                        // Add "previous" block based on our default duration to kick off with a semi-reasonable duration
                        let previous_block_ts = block_millis.saturating_sub(poll_timeout_duration.as_millis());
                        block_pid_cache.height.insert(previous_block_ht, previous_block_ts);
                    }
                    println!("block_pid_cache {:?} {:?}", block_pid_cache.current, block_pid_cache.height);

                    println!("all the things {:?} {:?} {:?}", block_height.clone().value(), now_millis, block_millis);
                    (next_duration, next_variance) = block_pid_cache.get_next(
                        now_millis,
                        block_height.clone().value(),
                        block_millis,
                    );

                    println!(
                        "Estimated next block time (duration: {:?}) variance {:?}",
                        next_duration, next_variance
                    );

                    yield block.into();
                }
                Ok(Err(err)) => {
                    debug!("Failed to get latest block: {}", err);
                    println!("Failed to get latest block: {}", err);
                }
                Err(err) => {
                    debug!("Timed out getting latest block: {}", err);
                    println!("Timed out getting latest block: {}", err);
                }
            }
            println!("HERE {:?}", rpc_request_start.elapsed());
            // sleep(Duration::from_secs_f64(poll_duration_secs)).await;
            // Wait
            // Make sure the block height changed, if not we need to get next height ASAP!
            if previous_block == block_height {
                println!("previous_block SAMMMEEEE {:?} waiting {:?}", block_height, next_variance);
                sleep(Duration::from_millis(next_variance)).await;
                previous_block = block_height;
            } else {
                println!("block_height NEWWWWWWWW {:?} waiting {:?}", block_height, next_duration);
                sleep(next_duration).await;
                previous_block = block_height;
            }
        }
    })
}
