use super::{log_tx_result, Action, DaoConfig, DaoPolicy, ProposalInput};
use near_sdk::{
    json_types::{Base58CryptoHash, U128},
    NearToken,
};
use near_workspaces::{
    result::{ExecutionResult, Value},
    Account, AccountId, Contract,
};

pub async fn storage_deposit(
    contract: &Contract,
    sender: &Account,
    account_id: Option<&AccountId>,
    registration_only: Option<bool>,
    deposit: Option<NearToken>,
) -> anyhow::Result<ExecutionResult<Value>> {
    log_tx_result(
        Some("storage_deposit"),
        sender
            .call(contract.id(), "storage_deposit")
            .args_json((account_id, registration_only))
            .deposit(deposit.unwrap_or(NearToken::from_millinear(10)))
            .max_gas()
            .transact()
            .await?,
    )
}

pub async fn mint(
    contract: &Contract,
    sender: &Account,
    account_id: &AccountId,
    amount: U128,
) -> anyhow::Result<ExecutionResult<Value>> {
    log_tx_result(
        Some("mint"),
        sender
            .call(contract.id(), "mint")
            .args_json((account_id, amount))
            .max_gas()
            .transact()
            .await?,
    )
}

pub async fn ft_transfer(
    sender: &Account,
    token_id: &AccountId,
    receiver_id: &AccountId,
    amount: u128,
) -> anyhow::Result<ExecutionResult<Value>> {
    log_tx_result(
        Some("ft_transfer"),
        sender
            .call(token_id, "ft_transfer")
            .args_json((receiver_id, U128(amount), Option::<String>::None))
            .max_gas()
            .deposit(NearToken::from_yoctonear(10))
            .transact()
            .await?,
    )
}

pub async fn new_dao(
    contract: &Contract,
    config: DaoConfig,
    policy: DaoPolicy,
) -> anyhow::Result<ExecutionResult<Value>> {
    log_tx_result(
        Some("DAO: new"),
        contract
            .call("new")
            .args_json((config, policy))
            .max_gas()
            .transact()
            .await?,
    )
}

pub async fn store_blob(
    sender: &Account,
    dao: &AccountId,
    blob: Vec<u8>,
    storage_cost: NearToken,
) -> anyhow::Result<Base58CryptoHash> {
    Ok(log_tx_result(
        Some("DAO: store_blob"),
        sender
            .call(dao, "store_blob")
            .args(blob)
            .max_gas()
            .deposit(storage_cost)
            .transact()
            .await?,
    )?
    .json()?)
}

pub async fn add_proposal(
    sender: &Account,
    dao: &AccountId,
    proposal: ProposalInput,
    deposit: Option<NearToken>,
) -> anyhow::Result<u64> {
    Ok(log_tx_result(
        Some("DAO: add_proposal"),
        sender
            .call(dao, "add_proposal")
            .args_json((proposal,))
            .max_gas()
            .deposit(deposit.unwrap_or(NearToken::from_near(1)))
            .transact()
            .await?,
    )?
    .json()?)
}

pub async fn act_proposal(
    sender: &Account,
    dao: &AccountId,
    proposal_id: u64,
    action: Action,
) -> anyhow::Result<ExecutionResult<Value>> {
    log_tx_result(
        Some("DAO: act_proposal"),
        sender
            .call(dao, "act_proposal")
            .args_json((proposal_id, action, None::<String>))
            .max_gas()
            .transact()
            .await?,
    )
}
