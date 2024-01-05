mod aurora;

use aurora_sdk_integration_tests::{
    aurora_engine, aurora_engine_sdk::types::near_account_to_evm_address,
    aurora_engine_types::types::Wei,
};
use near_sdk::json_types::U128;
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
