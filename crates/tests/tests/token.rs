mod aurora;
mod util;

use tokio::fs;
pub use util::*;

use aurora_sdk_integration_tests::{
    aurora_engine, aurora_engine_sdk::types::near_account_to_evm_address,
    aurora_engine_types::types::Wei,
};
use near_sdk::{env, json_types::U128, NearToken};
use near_workspaces::types::{KeyType, SecretKey};
use primitive_types::U256;

#[tokio::test]
async fn test_migration_success() -> anyhow::Result<()> {
    let mint_amount = 10_000;
    let aurora::AuroraInit {
        engine,
        aurora_wnear,
        shitzu_erc20,
        owner,
        contract,
        sol_contract,
        ..
    } = aurora::initialize_aurora(mint_amount, None).await?;
    let owner_address = near_account_to_evm_address(owner.id().as_bytes());

    let shitzuv1_balance = engine
        .erc20_balance_of(&shitzu_erc20, owner_address)
        .await?;
    assert_eq!(shitzuv1_balance.as_u128(), mint_amount);

    engine
        .mint_wnear(
            &aurora_wnear,
            sol_contract.address,
            10_000_000_000_000_000_000_000_000,
        )
        .await?;

    engine
        .mint_account(
            owner_address,
            0,
            Wei::new_u128(50_000_000_000_000_000_000_000),
        )
        .await?;

    aurora::approve_wnear(&engine, &owner, &sol_contract).await?;

    // approve Shitzuv1 for Solidity contract
    let result = engine
        .call_evm_contract_with(
            &owner,
            shitzu_erc20.address,
            shitzu_erc20.create_approve_call_bytes(sol_contract.address, U256::MAX),
            Wei::zero(),
        )
        .await?;
    aurora_engine::unwrap_success(result.status)?;

    aurora::migrate(&engine, &owner, &sol_contract, owner.id().to_string(), 1).await?;
    let balance: U128 = contract
        .view("ft_balance_of")
        .args_json((owner.id(),))
        .await?
        .json()?;
    assert_eq!(balance.0, 1);
    let shitzuv1_balance = engine
        .erc20_balance_of(&shitzu_erc20, owner_address)
        .await?;
    assert_eq!(shitzuv1_balance.as_u128(), mint_amount - 1);

    aurora::migrate(
        &engine,
        &owner,
        &sol_contract,
        owner.id().to_string(),
        1_000,
    )
    .await?;
    let balance: U128 = contract
        .view("ft_balance_of")
        .args_json((owner.id(),))
        .await?
        .json()?;
    assert_eq!(balance.0, 1_001);

    let shitzuv1_balance = engine
        .erc20_balance_of(&shitzu_erc20, owner_address)
        .await?;
    assert_eq!(shitzuv1_balance.as_u128(), mint_amount - 1_001);

    Ok(())
}

#[tokio::test]
async fn test_migration_missing_allowance() -> anyhow::Result<()> {
    let mint_amount = 10_000;
    let aurora::AuroraInit {
        engine,
        aurora_wnear,
        owner,
        contract,
        shitzu_erc20,
        sol_contract,
        ..
    } = aurora::initialize_aurora(mint_amount, None).await?;
    let owner_address = near_account_to_evm_address(owner.id().as_bytes());

    engine
        .mint_wnear(
            &aurora_wnear,
            sol_contract.address,
            10_000_000_000_000_000_000_000_000,
        )
        .await?;

    engine
        .mint_account(
            owner_address,
            0,
            Wei::new_u128(50_000_000_000_000_000_000_000),
        )
        .await?;

    aurora::approve_wnear(&engine, &owner, &sol_contract).await?;

    let res = aurora::migrate(&engine, &owner, &sol_contract, owner.id().to_string(), 1).await;
    assert!(res.is_err());
    let balance: U128 = contract
        .view("ft_balance_of")
        .args_json((owner.id(),))
        .await?
        .json()?;
    assert_eq!(balance.0, 0);
    let shitzuv1_balance = engine
        .erc20_balance_of(&shitzu_erc20, owner_address)
        .await?;
    assert_eq!(shitzuv1_balance.as_u128(), mint_amount);

    Ok(())
}

#[tokio::test]
async fn test_mint_check_predecessor() -> anyhow::Result<()> {
    let mint_amount = 10_000;
    let aurora::AuroraInit {
        engine,
        owner,
        contract,
        shitzu_erc20,
        ..
    } = aurora::initialize_aurora(mint_amount, None).await?;
    let owner_address = near_account_to_evm_address(owner.id().as_bytes());

    let res = owner
        .call(contract.id(), "mint")
        .args_json((owner.id(), 1_000))
        .transact()
        .await?
        .into_result();
    assert!(res.is_err());

    let balance: U128 = contract
        .view("ft_balance_of")
        .args_json((owner.id(),))
        .await?
        .json()?;
    assert_eq!(balance.0, 0);
    let shitzuv1_balance = engine
        .erc20_balance_of(&shitzu_erc20, owner_address)
        .await?;
    assert_eq!(shitzuv1_balance.as_u128(), mint_amount);

    Ok(())
}

#[tokio::test]
async fn test_upgrade_contract_via_dao() -> anyhow::Result<()> {
    let (worker, owner, contract) = aurora::initialize_contracts(
        Some("../../res/token.wasm"), // TODO set this to old contract
    )
    .await?;
    let council = worker.dev_create_account().await?;

    let dao_contract = worker
        .create_tla_and_deploy(
            "dao.test.near".parse()?,
            SecretKey::from_random(KeyType::ED25519),
            &fs::read("../../res/sputnik_dao.wasm").await?,
        )
        .await?
        .into_result()?;
    call::new_dao(
        &dao_contract,
        DaoConfig {
            name: "Shitzu".to_string(),
            purpose: "Shitzu 123".to_string(),
            metadata: "".to_string(),
        },
        DaoPolicy(vec![council.id().clone()]),
    )
    .await?;

    contract
        .call("new")
        .args_json((dao_contract.id(), owner.id()))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let user_0 = worker.dev_create_account().await?;
    let user_1 = worker.dev_create_account().await?;
    let user_2 = worker.dev_create_account().await?;

    tokio::try_join!(
        call::storage_deposit(&contract, &user_0, None, Some(true), None),
        call::storage_deposit(&contract, &user_1, None, Some(true), None),
        call::storage_deposit(&contract, &user_2, None, Some(true), None)
    )?;
    call::mint(&contract, &owner, user_0.id(), 100.into()).await?;
    call::mint(&contract, &owner, user_1.id(), 200.into()).await?;
    call::mint(&contract, &owner, user_2.id(), 300.into()).await?;

    let balance = view::ft_balance_of(&contract, user_0.id()).await?;
    assert_eq!(balance.0, 100);
    let balance = view::ft_balance_of(&contract, user_1.id()).await?;
    assert_eq!(balance.0, 200);
    let balance = view::ft_balance_of(&contract, user_2.id()).await?;
    assert_eq!(balance.0, 300);

    let total_supply = view::ft_total_supply(&contract).await?;
    assert_eq!(total_supply.0, 600);

    let blob = fs::read("../../res/token.wasm").await?;
    let storage_cost = ((blob.len() + 32) as u128) * env::storage_byte_cost().as_yoctonear();
    let hash = call::store_blob(
        &council,
        dao_contract.id(),
        blob,
        NearToken::from_yoctonear(storage_cost),
    )
    .await?;

    let proposal_id = call::add_proposal(
        &council,
        dao_contract.id(),
        ProposalInput {
            description: "upgrade contract".to_string(),
            kind: ProposalKind::UpgradeRemote {
                receiver_id: contract.id().clone(),
                method_name: "upgrade".to_string(),
                hash,
            },
        },
        None,
    )
    .await?;
    call::act_proposal(
        &council,
        dao_contract.id(),
        proposal_id,
        Action::VoteApprove,
    )
    .await?;

    let proposal_id = call::add_proposal(
        &council,
        dao_contract.id(),
        ProposalInput {
            description: "migrate contract".to_string(),
            kind: ProposalKind::FunctionCall {
                receiver_id: contract.id().clone(),
                actions: vec![ActionCall {
                    method_name: "migrate".to_string(),
                    args: vec![].into(),
                    deposit: 0.into(),
                    gas: 100_000_000_000_000.into(),
                }],
            },
        },
        None,
    )
    .await?;
    call::act_proposal(
        &council,
        dao_contract.id(),
        proposal_id,
        Action::VoteApprove,
    )
    .await?;

    let balance = view::ft_balance_of(&contract, user_0.id()).await?;
    assert_eq!(balance.0, 100);
    let balance = view::ft_balance_of(&contract, user_1.id()).await?;
    assert_eq!(balance.0, 200);
    let balance = view::ft_balance_of(&contract, user_2.id()).await?;
    assert_eq!(balance.0, 300);

    let total_supply = view::ft_total_supply(&contract).await?;
    assert_eq!(total_supply.0, 600);

    Ok(())
}
