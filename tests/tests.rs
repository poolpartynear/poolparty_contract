use near_sdk::{json_types::U128, NearToken};
use near_workspaces::{Account, Contract, DevNetwork, Worker};
use serde_json::json;

pub async fn init(
    worker: &Worker<impl DevNetwork>,
) -> Result<(Contract, Contract, Account, Account), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let contract = worker.dev_deploy(&contract_wasm).await?;

    let staking_contract = worker
        .dev_deploy(&std::fs::read("res/staking.wasm")?)
        .await?;

    let alice = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    // println!("{:?}", user_account.view_account().await?);
    // let outcome = user_account
    //     .call(staking_contract.id(), "deposit_and_stake")
    //     .args_json(json!({}))
    //     .deposit(NearToken::from_near(20))
    //     .transact()
    //     .await?;
    //
    let init_outcome = contract
        .call("new")
        .args_json(json!(
            { "external_pool": staking_contract.id(),
                "first_raffle": "1827381273",
                "min_to_raffle": NearToken::from_near(40),
                "max_to_raffle": NearToken::from_near(60),
                "min_deposit":  NearToken::from_near(1),
                "max_deposit":  NearToken::from_near(20),
                "epochs_wait":  "4",
                "time_between_raffles": "86400000000000"
            }
        ))
        .transact()
        .await?;

    assert!(init_outcome.is_success());

    // println!("{:?}", user_account.view_account().await?);

    // let staked: serde_json::Value = staking_contract
    //     .view("get_account")
    //     .args_json(json!({ "account_id": user_account.id() }))
    //     .await?
    //     .json()?;

    // println!("{:#}", staked);
    // let user_message_outcome = contract.view("get_greeting").args_json(json!({})).await?;
    // assert_eq!(user_message_outcome.json::<String>()?, "Hello World!");

    Ok((contract, staking_contract, alice, bob))
}

// Pool -----------------------------------------------------------
#[tokio::test]
async fn adds_tickets_to_user() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, _staking_contract, alice, _bob) = init(&worker).await?;

    let deposit_outcome = alice
        .call(contract.id(), "deposit_and_stake")
        .args_json(json!({}))
        .deposit(NearToken::from_near(2))
        .max_gas()
        .transact()
        .await?;
    assert!(deposit_outcome.is_success());

    let get_staked: U128 = alice
        .view(contract.id(), "get_staked_for")
        .args_json(json!({"user": alice.id()}))
        .await?
        .json()?;
    assert_eq!(
        NearToken::from_yoctonear(get_staked.0),
        NearToken::from_near(2)
    );

    let second_deposit_outcome = alice
        .call(contract.id(), "deposit_and_stake")
        .args_json(json!({}))
        .deposit(NearToken::from_near(2))
        .max_gas()
        .transact()
        .await?;
    assert!(second_deposit_outcome.is_success());

    let second_get_staked: U128 = alice
        .view(contract.id(), "get_staked_for")
        .args_json(json!({"user": alice.id()}))
        .await?
        .json()?;
    assert_eq!(
        NearToken::from_yoctonear(second_get_staked.0),
        NearToken::from_near(4)
    );

    Ok(())
}

// Emergency -----------------------------------------------------------
#[tokio::test]
async fn emergency() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, _staking_contract, alice, _bob) = init(&worker).await?;

    // User can't start or stop emergency
    let user_emergency_start = alice
        .call(contract.id(), "emergency_start")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(user_emergency_start.is_failure());

    let user_emergency_stop = alice
        .call(contract.id(), "emergency_stop")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(user_emergency_stop.is_failure());

    // Contract can start emergency
    let contract_emergency_start = contract
        .call("emergency_start")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(contract_emergency_start.is_success());

    // User can't deposit or unstake during emergency
    let deposit_during_emergency = alice
        .call(contract.id(), "deposit_and_stake")
        .args_json(json!({}))
        .deposit(NearToken::from_near(2))
        .max_gas()
        .transact()
        .await?;
    assert!(deposit_during_emergency.is_failure());

    let unstake_during_emergency = alice
        .call(contract.id(), "unstake")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?;
    assert!(unstake_during_emergency.is_failure());

    // User can't raffle during emergency
    let raffle_during_emergency = alice
        .call(contract.id(), "raffle")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?;
    assert!(raffle_during_emergency.is_failure());

    // User can't withdraw during emergency
    let withdraw_during_emergency = alice
        .call(contract.id(), "withdraw_all")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?;
    assert!(withdraw_during_emergency.is_failure());

    // Contract can stop emergency
    let contract_emergency_stop = contract
        .call("emergency_stop")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(contract_emergency_stop.is_success());

    Ok(())
}
