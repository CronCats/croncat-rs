use async_trait::async_trait;
use color_eyre::Report;
use cosmos_sdk_proto::cosmos::bank::v1beta1::query_client::QueryClient;
use cosmos_sdk_proto::cosmos::bank::v1beta1::QueryBalanceRequest;
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use tonic::transport::Channel;

#[derive(Clone, Debug)]
pub struct BankQueryClient {
    bank_query_client: QueryClient<Channel>,
    native_denom: String,
}

impl BankQueryClient {
    pub async fn new(grpc: String, native_denom: String) -> Result<Self, Report> {
        Ok(BankQueryClient {
            bank_query_client: QueryClient::connect(grpc).await?,
            native_denom,
        })
    }
}

pub trait GetBankQueryClient {
    fn bank_query_client(&self) -> BankQueryClient;
}

#[async_trait]
pub trait QueryBank {
    async fn query_native_balance(&self, account_addr: &str) -> Result<Coin, Report>;
    fn native_denom(&self) -> String;
}

#[async_trait]
impl<C> QueryBank for C
where
    C: GetBankQueryClient + std::marker::Sync,
{
    async fn query_native_balance(&self, account_addr: &str) -> Result<Coin, Report> {
        let BankQueryClient {
            bank_query_client: mut client,
            native_denom,
        } = self.bank_query_client();
        let request = QueryBalanceRequest {
            address: account_addr.to_owned(),
            denom: native_denom,
        };
        let balance = client
            .balance(request)
            .await?
            .into_inner()
            .balance
            .unwrap_or_default();

        Ok(balance)
    }

    fn native_denom(&self) -> String {
        self.bank_query_client().native_denom
    }
}

impl GetBankQueryClient for BankQueryClient {
    fn bank_query_client(&self) -> BankQueryClient {
        self.clone()
    }
}
