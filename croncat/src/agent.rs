//!
//! The croncat system daemon.
//!
use cosmos_sdk_proto::cosmwasm::wasm::v1::msg_client::MsgClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use crate::{
    env::Env,
    errors::Report,
    grpc,
    logging::info,
    streams::{agent, tasks, ws},
    tokio,
};

fn register_agent() {
}
