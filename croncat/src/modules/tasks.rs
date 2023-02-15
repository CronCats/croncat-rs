use cosm_orc::orchestrator::Address;
use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc,
};
// use cosmos_sdk_proto::tendermint::google::protobuf::Timestamp;
use croncat_sdk_agents::types::AgentStatus;
use croncat_sdk_tasks::msg::TasksQueryMsg;
use croncat_sdk_tasks::types::{TaskInfo, TaskResponse};
// use croncat_sdk_tasks::types::Boundary;
use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    errors::{eyre, Report},
    logging::info,
    monitor::ping_uptime_monitor,
    rpc::RpcClientService,
    utils::sum_num_tasks,
};
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::error;

use super::{agent::Agent, manager::Manager};

pub struct Tasks {
    pub client: RpcClientService,
    pub contract_addr: Address,
}

impl Tasks {
    pub async fn new(contract_addr: Address, client: RpcClientService) -> Result<Self, Report> {
        Ok(Self {
            client,
            contract_addr,
        })
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
                let from_index = from_index.clone();
                let limit = limit.clone();
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
                let start = start.clone();
                let from_index = from_index.clone();
                let limit = limit.clone();
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

    pub async fn fetch_all_evented_tasks(&self) -> Result<Vec<TaskInfo>, Report> {
        let mut evented_tasks = Vec::new();
        let mut start_index = 0;
        // NOTE: May need to support mut here if things get too crazy
        let from_index = 0;
        let limit = 100;
        loop {
            let current_iteration = self
                .get_evented_tasks(Some(start_index), Some(from_index), Some(limit))
                .await?;
            let last_iteration = current_iteration.len() < limit as usize;
            evented_tasks.extend(current_iteration);
            if last_iteration {
                break;
            }
            start_index += limit;
        }
        Ok(evented_tasks)
    }

    // TODO: Bring back!!!!!!!!!!!!!!!
    // pub async fn check_queries(
    //     &self,
    //     queries: Vec<CroncatQuery>,
    // ) -> Result<(bool, Option<u64>), Report> {
    //     let cw_rules_addr = {
    //         let cfg: GetConfigResponse = self.query_croncat(QueryMsg::GetConfig {}).await?;
    //         cfg.cw_rules_addr
    //     }
    //     .to_string();
    //     let res = self
    //         .rpc_client
    //         .call_wasm_query(
    //             Address::from_str(cw_rules_addr.as_str()).unwrap(),
    //             cw_rules_core::msg::QueryMsg::QueryConstruct(QueryConstruct { queries }),
    //         )
    //         .await?;
    //     Ok(res)
    // }
}

///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn tasks_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    manager_client: Manager,
    agent_client: Agent,
    block_status: Arc<Mutex<AgentStatus>>,
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
                    info!("Tasks: {:?}", tasks);
                    // TODO: Change this to batch, if possible!
                    for _ in 0..sum_num_tasks(&tasks) {
                        let tasks_failed = tasks_failed.clone();

                        match manager_client.proxy_call(None).await {
                            Ok(proxy_call_res) => {
                                info!("Finished task: {}", proxy_call_res.log);
                            }
                            Err(err) => {
                                tasks_failed.store(true, SeqCst);
                                error!("Something went wrong during proxy_call: {}", err);
                            }
                        }
                    }
                } else {
                    info!("No tasks for block (height: {})", block.header().height);
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

// TODO: Bring back - this needs major overhauls
// pub async fn queries_loop(
//     mut block_stream_rx: BlockStreamRx,
//     mut shutdown_rx: ShutdownRx,
//     client: RpcClientService,
//     block_status: Arc<Mutex<AgentStatus>>,
// ) -> Result<(), Report> {
//     let block_consumer_stream: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
//         while let Ok(block) = block_stream_rx.recv().await {
//             let tasks_with_queries = client
//                 .execute(|signer| async move {
//                     signer
//                         .fetch_queries()
//                         .await
//                         .map_err(|err| eyre!("Failed to fetch croncat query: {}", err))
//                 })
//                 .await?;

//             let locked_status = block_status.lock().await;
//             let is_active = *locked_status == AgentStatus::Active;
//             // unlocking it ASAP
//             std::mem::drop(locked_status);
//             if is_active {
//                 let tasks_failed = Arc::new(AtomicBool::new(false));
//                 let time: Timestamp = block.header().time.into();
//                 let time_nanos = time.seconds as u64 * 1_000_000_000 + time.nanos as u64;

//                 for task in tasks_with_queries.iter() {
//                     let in_boundary = match task.boundary {
//                         Some(Boundary::Height { start, end }) => {
//                             let height = block.header().height.value();
//                             start.map_or(true, |s| s.u64() >= height)
//                                 && end.map_or(true, |e| e.u64() <= height)
//                         }
//                         Some(Boundary::Time { start, end }) => {
//                             start.map_or(true, |s| s.nanos() >= time_nanos)
//                                 && end.map_or(true, |e| e.nanos() >= time_nanos)
//                         }
//                         None => true,
//                     };
//                     if in_boundary {
//                         let (queries_ready, _) = client
//                             .execute(|signer| async move {
//                                 signer
//                                     .check_queries(
//                                         task.queries
//                                             .clone()
//                                             .ok_or_else(|| eyre!("No croncat query"))?,
//                                     )
//                                     .await
//                                     .map_err(|err| eyre!("Failed to query croncat query: {}", err))
//                             })
//                             .await?;
//                         if queries_ready {
//                             client
//                                 .execute(|signer| {
//                                     let tasks_failed = tasks_failed.clone();
//                                     async move {
//                                         match signer.proxy_call(Some(task.task_hash.clone())).await
//                                         {
//                                             Ok(proxy_call_res) => {
//                                                 info!("Finished task: {}", proxy_call_res.log);
//                                             }
//                                             Err(err) => {
//                                                 tasks_failed.store(true, SeqCst);
//                                                 error!(
//                                                     "Something went wrong during proxy_call: {}",
//                                                     err
//                                                 );
//                                             }
//                                         }

//                                         Ok(())
//                                     }
//                                 })
//                                 .await?;
//                         }
//                     }
//                 }

//                 if !tasks_failed.load(SeqCst) {
//                     ping_uptime_monitor().await;
//                 }
//             }
//         }

//         Ok(())
//     });
//     tokio::select! {
//         _ = block_consumer_stream => {}
//         _ = shutdown_rx.recv() => {}
//     }

//     Ok(())
// }
