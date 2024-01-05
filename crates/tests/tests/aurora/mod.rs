use aurora_sdk_integration_tests::{
    aurora_engine::{self, erc20::ERC20, AuroraEngine, ContractInput},
    aurora_engine_sdk::types::near_account_to_evm_address,
    aurora_engine_types::types::{Address, Wei},
    ethabi::{self, Uint},
    utils::{ethabi::DeployedContract, forge},
    wnear::{self, Wnear},
};
use near_sdk::NearToken;
use near_workspaces::{
    network::Sandbox,
    types::{KeyType, SecretKey},
    Account, Contract, Worker,
};
use serde_json::json;
use std::path::Path;
use tokio::fs;

pub struct AuroraInit {
    pub worker: Worker<Sandbox>,
    pub engine: AuroraEngine,
    pub aurora_wnear: Wnear,
    pub shitzu_erc20: ERC20,
    pub owner: Account,
    pub contract: Contract,
    pub sol_contract: DeployedContract,
    pub near_representative_id: String,
}

pub async fn initialize_aurora(
    mint_amount: u128,
    contract_path: Option<&str>,
) -> anyhow::Result<AuroraInit> {
    let (worker, owner, contract) = initialize_contracts().await?;
    let engine = aurora_engine::deploy_latest(&worker).await.unwrap();
    let aurora_wnear = wnear::Wnear::deploy(&worker, &engine).await.unwrap();

    let wasm = fs::read(contract_path.unwrap_or("../../res/test_token.wasm")).await?;
    let shitzu_old = worker.dev_deploy(wasm.as_slice()).await?;
    shitzu_old
        .call("new")
        .args_json(("SHITZUv1", "SHITZU", None::<String>, 18))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let shitzu_erc20 = engine.bridge_nep141(shitzu_old.id()).await?;
    shitzu_old
        .call("storage_deposit")
        .args_json((owner.id(), true))
        .max_gas()
        .deposit(NearToken::from_millinear(100))
        .transact()
        .await?
        .into_result()?;
    shitzu_old
        .call("storage_deposit")
        .args_json((engine.inner.id(), true))
        .max_gas()
        .deposit(NearToken::from_millinear(100))
        .transact()
        .await?
        .into_result()?;
    shitzu_old
        .call("mint")
        .args_json((owner.id(), mint_amount.to_string()))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let owner_address = near_account_to_evm_address(owner.id().as_bytes());
    owner
        .call(shitzu_old.id(), "ft_transfer_call")
        .args(
            json!({
                "receiver_id": engine.inner.id(),
                "amount": 10_000.to_string(),
                "memo": "null",
                "msg": owner_address.encode()
            })
            .to_string()
            .as_bytes()
            .to_vec(),
        )
        .max_gas()
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?
        .into_result()?;

    let sol_contract = deploy_sol_contract(
        &engine,
        &owner,
        aurora_wnear.aurora_token.address,
        shitzu_erc20.address,
        contract.id().to_string(),
    )
    .await;
    let near_representative_id = format!("{}.{}", sol_contract.address.encode(), engine.inner.id());

    contract
        .call("new")
        .args_json((&near_representative_id,))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    Ok(AuroraInit {
        worker,
        engine,
        aurora_wnear,
        shitzu_erc20,
        owner,
        contract,
        sol_contract,
        near_representative_id,
    })
}

async fn initialize_contracts() -> anyhow::Result<(Worker<Sandbox>, Account, Contract)> {
    let worker = near_workspaces::sandbox().await?;

    let owner = worker.dev_create_account().await?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let contract = worker
        .create_tla_and_deploy(
            "token.test.near".parse()?,
            key,
            &fs::read("../../res/token.wasm").await?,
        )
        .await?
        .into_result()?;

    Ok((worker, owner, contract))
}

async fn deploy_sol_contract(
    engine: &AuroraEngine,
    proxy_account: &Account,
    wnear_address: Address,
    shitzu_address: Address,
    shitzu_near_id: String,
) -> DeployedContract {
    let contract_path = "../../solidity";
    let aurora_sdk_path = Path::new("../../crates/aurora-sdk/aurora-solidity-sdk");
    let codec_lib = forge::deploy_codec_lib(&aurora_sdk_path, engine)
        .await
        .unwrap();
    let utils_lib = forge::deploy_utils_lib(&aurora_sdk_path, engine)
        .await
        .unwrap();
    let aurora_sdk_lib =
        forge::deploy_aurora_sdk_lib(&aurora_sdk_path, engine, codec_lib, utils_lib)
            .await
            .unwrap();
    let constructor = forge::forge_build(
        contract_path,
        &[format!(
            "aurora-sdk/AuroraSdk.sol:AuroraSdk:0x{}",
            aurora_sdk_lib.encode()
        )],
        &["out", "ShitzuMigrate.sol", "ShitzuMigrate.json"],
    )
    .await
    .unwrap();
    let deploy_bytes = constructor.create_deploy_bytes_with_args(&[
        ethabi::Token::Address(wnear_address.raw()),
        ethabi::Token::Address(shitzu_address.raw()),
        ethabi::Token::String(shitzu_near_id),
    ]);
    let address = engine
        .deploy_evm_contract_with(proxy_account, deploy_bytes)
        .await
        .unwrap();
    constructor.deployed_at(address)
}

pub async fn approve_wnear(
    engine: &AuroraEngine,
    account: &Account,
    sol_contract: &DeployedContract,
) -> anyhow::Result<()> {
    println!("approve wNear for representative account");
    let result = engine
        .call_evm_contract_with(
            account,
            sol_contract.address,
            ContractInput(sol_contract.create_call_method_bytes_with_args("approveWNEAR", &[])),
            Wei::zero(),
        )
        .await?;
    aurora_engine::unwrap_success(result.status)?;
    Ok(())
}

pub async fn migrate<T: Into<Uint>>(
    engine: &AuroraEngine,
    account: &Account,
    sol_contract: &DeployedContract,
    account_id: String,
    amount: T,
) -> anyhow::Result<()> {
    println!("migrate");
    let result = engine
        .call_evm_contract_with(
            account,
            sol_contract.address,
            ContractInput(sol_contract.create_call_method_bytes_with_args(
                "migrate",
                &[
                    ethabi::Token::String(account_id),
                    ethabi::Token::Uint(amount.into()),
                ],
            )),
            Wei::zero(),
        )
        .await?;
    aurora_engine::unwrap_success(result.status)?;
    Ok(())
}
