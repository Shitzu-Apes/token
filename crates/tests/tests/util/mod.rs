pub mod call;
pub mod event;
pub mod view;

use near_sdk::json_types::{Base58CryptoHash, Base64VecU8, U128, U64};
use near_workspaces::{
    result::{ExecutionFinalResult, ExecutionResult, Value, ViewResultDetails},
    AccountId,
};
use owo_colors::OwoColorize;
use serde::Serialize;

#[macro_export]
macro_rules! print_log {
    ( $x:expr, $($y:expr),+ ) => {
        let thread_name = std::thread::current().name().unwrap().to_string();
        if thread_name == "main" {
            println!($x, $($y),+);
        } else {
            let mut s = format!($x, $($y),+);
            s = s.split('\n').map(|s| {
                let mut pre = "    ".to_string();
                pre.push_str(s);
                pre.push('\n');
                pre
            }).collect::<String>();
            println!(
                "{}\n{}",
                thread_name.bold(),
                &s[..s.len() - 1],
            );
        }
    };
}

#[derive(Serialize)]
pub struct DaoConfig {
    pub name: String,
    pub purpose: String,
    pub metadata: String,
}

#[derive(Serialize)]
pub struct DaoPolicy(pub Vec<AccountId>);

#[derive(Serialize)]
pub struct ProposalInput {
    pub description: String,
    pub kind: ProposalKind,
}

#[allow(unused)]
#[derive(Serialize)]
pub enum ProposalKind {
    /// Change the DAO config.
    ChangeConfig { config: DaoConfig },
    /// Change the full policy.
    ChangePolicy { policy: DaoPolicy },
    /// Add member to given role in the policy. This is short cut to updating the whole policy.
    AddMemberToRole { member_id: AccountId, role: String },
    /// Remove member to given role in the policy. This is short cut to updating the whole policy.
    RemoveMemberFromRole { member_id: AccountId, role: String },
    /// Calls `receiver_id` with list of method names in a single promise.
    /// Allows this contract to execute any arbitrary set of actions in other contracts.
    FunctionCall {
        receiver_id: AccountId,
        actions: Vec<ActionCall>,
    },
    /// Upgrade this contract with given hash from blob store.
    UpgradeSelf { hash: Base58CryptoHash },
    /// Upgrade another contract, by calling method with the code from given hash from blob store.
    UpgradeRemote {
        receiver_id: AccountId,
        method_name: String,
        hash: Base58CryptoHash,
    },
}

#[derive(Serialize)]
pub struct ActionCall {
    pub method_name: String,
    pub args: Base64VecU8,
    pub deposit: U128,
    pub gas: U64,
}

#[allow(unused)]
#[derive(Serialize)]
pub enum Action {
    AddProposal,
    RemoveProposal,
    VoteApprove,
    VoteReject,
    VoteRemove,
    Finalize,
    MoveToHub,
}

pub fn log_tx_result(
    ident: Option<&str>,
    res: ExecutionFinalResult,
) -> anyhow::Result<ExecutionResult<Value>> {
    for failure in res.receipt_failures() {
        print_log!("{:#?}", failure.bright_red());
    }
    for outcome in res.receipt_outcomes() {
        if !outcome.logs.is_empty() {
            for log in outcome.logs.iter() {
                if log.starts_with("EVENT_JSON:") {
                    let event: event::ContractEvent =
                        serde_json::from_str(&log.replace("EVENT_JSON:", ""))?;
                    print_log!(
                        "{}: {}\n{}",
                        "account".bright_cyan(),
                        outcome.executor_id,
                        event
                    );
                } else {
                    print_log!("{}", log.bright_yellow());
                }
            }
        }
    }
    if let Some(ident) = ident {
        print_log!(
            "{} gas burnt: {:.3} {}",
            ident.italic(),
            (res.total_gas_burnt.as_gas() as f64 / 1_000_000_000_000.)
                .bright_magenta()
                .bold(),
            "TGas".bright_magenta().bold()
        );
    }
    Ok(res.into_result()?)
}

pub fn log_view_result(res: ViewResultDetails) -> anyhow::Result<ViewResultDetails> {
    if !res.logs.is_empty() {
        for log in res.logs.iter() {
            print_log!("{}", log.bright_yellow());
        }
    }
    Ok(res)
}
