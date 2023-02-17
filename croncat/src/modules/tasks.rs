use cosm_orc::orchestrator::Address;
use cosmwasm_std::from_binary;
use std::{sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc,
}, str::FromStr};
// use cosmos_sdk_proto::tendermint::google::protobuf::Timestamp;
use croncat_sdk_agents::types::AgentStatus;
use croncat_sdk_tasks::msg::TasksQueryMsg;
use croncat_sdk_tasks::types::{TaskInfo, TaskResponse, CroncatQuery};
use mod_sdk::types::QueryResponse;
// use croncat_sdk_tasks::types::Boundary;
use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    errors::{eyre, Report},
    logging::{info, debug},
    monitor::ping_uptime_monitor,
    rpc::RpcClientService,
    store::tasks::LocalEventStorage,
};
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::error;

use super::{agent::Agent, manager::Manager};

pub struct Tasks {
    pub client: RpcClientService,
    pub contract_addr: Address,
    pub store: LocalEventStorage,
    // for helping with batch query validation
    pub generic_querier_addr: Address,
}

// FLOW:
// - check if local cache has tasks ready, if at all
//   - if no tasks, go get from chain - using current chain context
//   - if tasks, load into local cache & storage
// - return known tasks
//
// NOTE: why "events"? because these are task data that get triggered upon certain events
//
// Example Data:
// {
//   // when this cache should get removed/updated
//   expires: 1696407069536,
//   // Index based tasks
//   events: {
//     // Index here is the starting block height based on boundary (if task has it)
//     // NOTE: Non-boundary tasks will always have index be zero
//     300001: {
//         // key: Task Hash, value: Task
//         "osmosistestnet:f9a4e4e6f0dc427db55086fc4dba14f3244b392b4c5b46b72": {
//             ...Task Data
//         }
//     }
//   }
// }

impl Tasks {
    pub async fn new(contract_addr: Address, client: RpcClientService, generic_querier_addr: Address) -> Result<Self, Report> {
        Ok(Self {
            client,
            contract_addr,
            store: LocalEventStorage::default(),
            generic_querier_addr,
        })
    }

    // get cached events, or get new & put into storage
    // NOTE: Result returns if it was reloaded or not
    pub async fn load(&mut self) -> Result<bool, Report> {
        let b = if self.store.get().is_some() {
            // Have the unexpired cache data, wooooot!
            false
        } else {
            self.load_all_evented_tasks().await?;
            true
        };

        // only need to make sure we loaded y'all
        Ok(b)
    }

    // stats helper
    pub async fn get_stats(&self) -> Result<(u64, u64), Report> {
        Ok(self.store.get_stats())
    }

    pub async fn get_all(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let response: Vec<TaskResponse> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let from_index = from_index;
                let limit = limit;
                async move {
                    querier
                        .query_croncat(
                            TasksQueryMsg::Tasks { from_index, limit },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }

    // returns the range IDs needed for evented pagination
    pub async fn get_evented_ids(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<u64>, Report> {
        let res: Vec<u64> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let from_index = from_index;
                let limit = limit;
                async move {
                    querier
                        .query_croncat(
                            TasksQueryMsg::EventedIds {
                                from_index,
                                limit,
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(res)
    }

    // get evented tasks with pagination
    pub async fn get_evented_tasks(
        &self,
        start: Option<u64>,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<TaskInfo>, Report> {
        let res: Vec<TaskInfo> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let start = start;
                let from_index = from_index;
                let limit = limit;
                async move {
                    querier
                        .query_croncat(
                            TasksQueryMsg::EventedTasks {
                                start,
                                from_index,
                                limit,
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(res)
    }

    pub async fn load_all_evented_tasks(&mut self) -> Result<(), Report> {
        let mut evented_ids: Vec<u64> = Vec::new();
        let mut from_index = 0;
        let limit = 100;

        // Step 1: Get all the ids
        loop {
            let current_iteration = self
                .get_evented_ids(Some(from_index), Some(limit))
                .await?;
            let last_iteration = current_iteration.len() < limit as usize;
            evented_ids.extend(current_iteration);
            if last_iteration {
                break;
            }
            from_index += limit;
        }

        // Step 1: Get all the data from ids
        for id in evented_ids {
            let mut evented_tasks: Vec<(String, TaskInfo)> = Vec::new();
            from_index = 0;
            loop {
                // pagination at specific index
                let current_iteration = self
                    .get_evented_tasks(Some(id), Some(from_index), Some(limit))
                    .await?;
                let last_iteration = current_iteration.len() < limit as usize;
                evented_tasks.extend(current_iteration.into_iter().map(|t| (t.task_hash.clone(), t)).collect::<Vec<(String, TaskInfo)>>());
                if last_iteration {
                    break;
                }
                from_index += limit;
            }

            // update storage
            self.store.insert(id, evented_tasks)?;
        }

        Ok(())
    }

    // submit the same queries that will re-evaluate on-chain
    // Just need to get all to eval "true" to submit to the chain
    pub async fn validate_queries(
        &self,
        queries: Vec<CroncatQuery>,
    ) -> Result<(), Report> {
        // Process all the queries
        let mut filtered_q = queries.clone();
        filtered_q.retain(|q| q.check_result);

        // TODO: This needs to change to be BATCH query! Too much latency here...
        for q in filtered_q.iter() {
            let res: Result<QueryResponse, Report> = self
                .client
                .query(move |querier| {
                    let deser_msg: CroncatQuery = from_binary(&q.msg).expect("Deser query msg failed");
                    async move {
                        querier
                            .query_croncat(
                                deser_msg,
                                Some(Address::from_str(q.contract_addr.as_str())?),
                            )
                            .await
                    }
                })
                .await;
            
            // Eval if result false, if so break!
            match res {
                // likely this was because the response payload didnt match
                Err(_) => { break; }
                Ok(data) => {
                    if !data.result { break; }
                }
            }
        }

        Ok(())
    }
}

///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn scheduled_tasks_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    block_status: Arc<Mutex<AgentStatus>>,
    chain_id: Arc<String>,
    agent_client: Arc<Agent>,
    manager_client: Arc<Manager>,
    tasks_client: Arc<Tasks>,
) -> Result<(), Report> {
    let block_consumer_stream: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            let locked_status = block_status.lock().await;
            let is_active = *locked_status == AgentStatus::Active;
            // unlocking it ASAP
            std::mem::drop(locked_status);
            if is_active {
                let tasks_failed = Arc::new(AtomicBool::new(false));
                let account_addr = agent_client.account_id();
                let tasks = agent_client
                    .get_tasks(account_addr.as_str())
                    .await
                    .map_err(|err| eyre!("Failed to get agent tasks: {}", err))?;

                if let Some(tasks) = tasks {
                    // also get info about evented stats
                    let stats = tasks_client.get_stats().await?;

                    info!(
                        "[{}] Block {} :: Block Tasks: {}, Cron Tasks: {}, Evented Unbounded: {}, Evented Indexed: {}",
                        chain_id,
                        block.header().height,
                        tasks.stats.num_block_tasks,
                        tasks.stats.num_cron_tasks,
                        stats.0,
                        stats.1,
                    );

                    // TODO: Limit batches to max gas 3_000_000-6_000_000 (also could be set per-chain since stargaze has higher limits for example)
                    // Batch proxy_call's for known task counts
                    let tasks_failed = tasks_failed.clone();
                    let task_count: usize = u64::from(tasks.stats.num_block_tasks.saturating_add(tasks.stats.num_cron_tasks)) as usize;
                    match manager_client.proxy_call_batch(task_count).await {
                        Ok(pc_res) => {
                            debug!("Result: {:?}", pc_res.res.log);
                            info!("Finished task batch - TX: {}, Blk: {}, Evts: {}", pc_res.tx_hash, pc_res.height, pc_res.events.len());
                        }
                        Err(err) => {
                            tasks_failed.store(true, SeqCst);
                            error!("Something went wrong during proxy_call_batch: {}", err);
                        }
                    }
                } else {
                    info!(
                        "[{}] No tasks for block (height: {})",
                        chain_id,
                        block.header().height
                    );
                }

                if !tasks_failed.load(SeqCst) {
                    ping_uptime_monitor().await;
                }
            }
        }

        Ok(())
    });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}

pub async fn evented_tasks_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    block_status: Arc<Mutex<AgentStatus>>,
    _chain_id: Arc<String>,
    _agent_client: Arc<Agent>,
    _manager_client: Arc<Manager>,
    _tasks_client: Arc<Tasks>,
) -> Result<(), Report> {
    let block_consumer_stream: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(_block) = block_stream_rx.recv().await {
            let locked_status = block_status.lock().await;
            let is_active = *locked_status == AgentStatus::Active;
            // unlocking it ASAP
            std::mem::drop(locked_status);
            if is_active {
                let tasks_failed = Arc::new(AtomicBool::new(false));

                // FLOW:
                // - get the stack of ranged evented tasks
                // - organize into batched queries (batch of batched task level queries)
                // - evaluate queries, filter batch down to valid tasks
                // - batch execute valid tasks

                // Stack 0: Unbounded evented tasks
                // - These will get queried every block
                // let unbounded = task_client.unbounded().await?;

                // Stack 1: Ranged evented tasks
                // - These will get queried every block, as long as the index is lt block height/timestamp
                // let unbounded_height = task_client.ranged(block.header().height).await?;
                // let unbounded_timestamp = task_client.ranged(block.header().time).await?;


                // let account_addr = agent_client.account_id();
                // let tasks = agent_client
                //     .get_tasks(account_addr.as_str())
                //     .await
                //     .map_err(|err| eyre!("Failed to get agent tasks: {}", err))?;

                // if let Some(tasks) = tasks {
                //     // TODO: Change this to batch, if possible!
                //     for _ in 0..sum_num_tasks(&tasks) {
                //         let tasks_failed = tasks_failed.clone();

                //         match manager_client.proxy_call(None).await {
                //             Ok(proxy_call_res) => {
                //                 info!("Finished task: {}", proxy_call_res.res.log);
                //             }
                //             Err(err) => {
                //                 tasks_failed.store(true, SeqCst);
                //                 error!("Something went wrong during proxy_call: {}", err);
                //             }
                //         }
                //     }
                // }

                if !tasks_failed.load(SeqCst) {
                    ping_uptime_monitor().await;
                }
            }
        }

        Ok(())
    });
    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
