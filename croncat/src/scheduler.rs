use std::str::FromStr;
use std::{fmt::Debug, ops::Deref};

use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use serde::{de::Visitor, Deserialize, Serialize};

///
/// Wrap the Schedule object from cron_schedule.
///
#[derive(Debug)]
pub struct Schedule(cron_schedule::Schedule);

impl Deref for Schedule {
    type Target = cron_schedule::Schedule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for Schedule {
    type Err = cron_schedule::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match cron_schedule::Schedule::from_str(s) {
            Ok(schedule) => Ok(Schedule(schedule)),
            Err(err) => Err(err),
        }
    }
}

///
/// Handle serialization for the wrapped schedule object
///
impl Serialize for Schedule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

struct ScheduleVisitor;

impl<'de> Visitor<'de> for ScheduleVisitor {
    type Value = Schedule;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A cron_schedule string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match cron_schedule::Schedule::from_str(v.as_str()) {
            Ok(schedule) => Ok(Schedule(schedule)),
            Err(_) => Err(E::custom(format!("Invalid schedule string: {}", v))),
        }
    }
}

///
/// Handle de-serialization of a schedule string into a Schedule.
///
impl<'de> Deserialize<'de> for Schedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ScheduleVisitor)
    }
}

///
/// Proxy object for Coin.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractCallValue {
    pub denom: String,
    pub amount: String,
}

impl Into<Coin> for &ContractCallValue {
    fn into(self) -> Coin {
        Coin {
            denom: self.denom.clone(),
            amount: self.amount.clone(),
        }
    }
}

///
/// Proxy object for MsgExecuteContract.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractCall {
    pub sender: String,
    pub contract: String,
    pub msg: Vec<u8>,
    pub funds: Vec<ContractCallValue>,
}

impl Into<MsgExecuteContract> for ContractCall {
    fn into(self) -> MsgExecuteContract {
        MsgExecuteContract {
            sender: self.sender,
            contract: self.contract,
            msg: self.msg,
            funds: self.funds.iter().map(|c| c.into()).collect(),
        }
    }
}

///
/// An action and sub-actions that will run when our schedule is met.
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub contract_call: ContractCall,
    pub sub_actions: Vec<Action>,
}

///
/// A task
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub id: String,
    pub account_id: String,
    pub schedule: Schedule,
    pub actions: Vec<Action>,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

///
/// The scheduler itself
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Scheduler {
    pub tasks: Vec<Task>,
}
