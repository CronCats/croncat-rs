use cosmos_sdk_proto::cosmos::vesting::v1beta1::PeriodicVestingAccount;
use cosmos_sdk_proto::cosmwasm::wasm::v1::{QueryContractInfoRequest, QuerySmartContractStateRequest, QuerySmartContractStateResponse, QueryContractInfoResponse};
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::{QueryClient};
use cosmos_sdk_proto::cosmos::auth::v1beta1::{BaseAccount, QueryAccountRequest, QueryAccountResponse};

use serde_json;

use tonic::transport::Channel;


pub async fn get_contract_info(channel: Channel, address: String) -> anyhow::Result<QueryContractInfoResponse> {
    let res = QueryClient::new(channel).contract_info(QueryContractInfoRequest { address: address }).await?.into_inner();
    //println!("{:?}", &res);
    Ok(res)
}

pub async fn get_smart_contract_state<T: ?Sized + serde::Serialize>(channel: Channel, address: String, query_msg: &T) -> anyhow::Result<QuerySmartContractStateResponse> {
    let res = QueryClient::new(channel).smart_contract_state(QuerySmartContractStateRequest { address, query_data: serde_json::to_vec(query_msg)? }).await?.into_inner();
    //println!("{:?}", &res);
    Ok(res)
}
