pub use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient as AuthQueryClient;
pub use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient as WasmQueryClient;

mod query_handlers;
use query_handlers::{auth_query, bank_query, wasm_query};

pub mod full_client;
pub mod query_client;
mod wasm_execute;

pub use auth_query::{GetAuthQueryClient, QueryBaseAccount};
pub use bank_query::{BankQueryClient, GetBankQueryClient, QueryBank};
pub use wasm_query::{GetWasmQueryClient, QuerySmartContract};
