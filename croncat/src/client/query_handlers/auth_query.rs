use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient;
use cosmos_sdk_proto::cosmos::auth::v1beta1::{BaseAccount, QueryAccountRequest};
use prost::Message;
use tonic::transport::Channel;

pub trait GetAuthQueryClient {
    fn auth_query_client(&self) -> QueryClient<Channel>;
}

#[async_trait]
pub trait QueryBaseAccount {
    async fn query_base_account(&self, account_addr: String) -> Result<BaseAccount, Report>;
}

#[async_trait]
impl<C> QueryBaseAccount for C
where
    C: GetAuthQueryClient + std::marker::Sync,
{
    async fn query_base_account(&self, account_addr: String) -> Result<BaseAccount, Report> {
        let mut client = self.auth_query_client();
        let request = QueryAccountRequest {
            address: account_addr,
        };
        let res = client.account(request).await?.into_inner();
        let account = res.account.ok_or(eyre!("failed base account query"))?;
        let base_account = BaseAccount::decode(account.value.as_slice())?;
        Ok(base_account)
    }
}
