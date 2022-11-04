use color_eyre::eyre::eyre;
use color_eyre::Report;
use cosmos_chain_registry::ChainInfo;
use cosmos_sdk_proto::cosmos::auth::v1beta1::BaseAccount;
use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient;
use cosmos_sdk_proto::cosmos::tx::v1beta1::SimulateRequest;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::rpc::HttpClient;
use cosmrs::tx::{self, Msg};
use cosmrs::tx::{Fee, Raw, SignDoc, SignerInfo};
use cosmrs::{Coin, Denom};
use serde::Serialize;
use tendermint_rpc::endpoint::broadcast::tx_commit::Response;
use tonic::transport::Channel;

pub fn generate_wasm_body(
    sender: &str,
    contract_name: &str,
    msg: &impl Serialize,
) -> Result<tx::Body, Report> {
    let body = tx::Body::new(
        vec![MsgExecuteContract {
            sender: sender.parse()?,
            contract: contract_name.parse()?,
            msg: serde_json::to_vec(msg)?,
            funds: vec![],
        }
        .to_any()?],
        "MEOW! Luvv, Cron.Cat",
        0u16,
    );
    Ok(body)
}

pub fn prepare_send(
    tx: &tx::Body,
    chain_info: &ChainInfo,
    key: &SigningKey,
    base_account: &BaseAccount,
    fee: Fee,
) -> Result<Raw, Report> {
    let auth_info =
        SignerInfo::single_direct(Some(key.public_key()), base_account.sequence).auth_info(fee);
    let sign_doc = SignDoc::new(
        tx,
        &auth_info,
        &chain_info.chain_id.parse()?,
        base_account.account_number,
    )?;
    let tx_raw = sign_doc.sign(key)?;
    Ok(tx_raw)
}

pub fn prepare_simulate_tx(
    tx: &tx::Body,
    chain_info: &ChainInfo,
    key: &SigningKey,
    base_account: &BaseAccount,
) -> Result<Raw, Report> {
    let denom: Denom = chain_info.fees.fee_tokens[0].denom.parse()?;
    let auth_info = SignerInfo::single_direct(Some(key.public_key()), base_account.sequence)
        .auth_info(Fee::from_amount_and_gas(
            Coin {
                denom,
                amount: 0u64.into(),
            },
            0u64,
        ));

    let sign_doc = SignDoc::new(
        tx,
        &auth_info,
        &chain_info.chain_id.parse()?,
        base_account.account_number,
    )?;

    let tx_raw = sign_doc.sign(key)?;
    Ok(tx_raw)
}

/// Thanks `cosm-orc` author(@de-husk) for this simulate-gas method:
/// https://github.com/de-husk/cosm-orc/blob/834e681b0e8371e2bae07aff18a0fd79171088b5/src/client/cosm_client.rs#L276
pub async fn simulate_gas_fee(
    mut client: ServiceClient<Channel>,
    tx_raw: Raw,
    denom: &String,
    gas_prices: f32,
    gas_adjustment: f32,
) -> Result<Fee, Report> {
    let denom: Denom = denom.parse()?;
    let gas_info = client
        .simulate(SimulateRequest {
            tx_bytes: tx_raw.to_bytes()?,
            ..Default::default()
        })
        .await?
        .into_inner()
        .gas_info
        .ok_or_else(|| eyre!("No gas info in simulate response"))?;

    //  TODO: (REFACTOR) This is a hack to get the gas price from the chain config. We should be able to get this from the chain itself.
    let gas_limit = (gas_info.gas_used as f32 * gas_adjustment).ceil();
    let amount = Coin {
        denom: denom.clone(),
        amount: ((gas_limit * gas_prices).ceil() as u64).into(),
    };

    Ok(Fee::from_amount_and_gas(amount, gas_limit as u64))
}

pub async fn send_tx(client: &HttpClient, tx: Raw) -> Result<Response, Report> {
    let res = tx.broadcast_commit(client).await?;
    if res.check_tx.code.is_err() {
        Err(eyre!("{:?}", res.check_tx))
    } else if res.deliver_tx.code.is_err() {
        Err(eyre!("{:?}", res.deliver_tx))
    } else {
        Ok(res)
    }
}
