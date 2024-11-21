use chrono::Utc;
use near_workspaces::{AccountId, network::Sandbox, types::NearToken, Account, Contract, Worker};
use poolparty::{pool::Pool, UserInfo};
use serde_json::json;

#[tokio::test]
async fn test_contract_is_operational() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;

    let root = sandbox.root_account().unwrap();

    // who controls the reserve
    let guardian = root
        .create_subaccount("guardian")
        .initial_balance(NearToken::from_near(20))
        .transact()
        .await?
        .unwrap();

    // the pool party contract
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // the mock validator
    let staking_contract = sandbox
        .dev_deploy(&std::fs::read(
            "./tests/mock-validator/mock-validator.wasm",
        )?)
        .await?;

    let now = Utc::now().timestamp();
    let one_minute = 60 * 1000000000;
    let a_minute_from_now = now * 1000000000 + one_minute;

    let init = contract
        .call("new")
        .args_json(json!(
            {
                "guardian": guardian.id(),
                "external_pool": staking_contract.id(),
                "first_raffle": a_minute_from_now.to_string(),
                "time_between_raffles": one_minute.to_string(),
            }
        ))
        .transact()
        .await?;

    assert!(init.is_success());

    let ana = root
        .create_subaccount("ana")
        .initial_balance(NearToken::from_near(60))
        .transact()
        .await?
        .unwrap();
    let bob = root
        .create_subaccount("bob")
        .initial_balance(NearToken::from_near(20))
        .transact()
        .await?
        .unwrap();

    // Begin tests
    test_deposit(&ana, &bob, &guardian, &contract).await?;
    test_raffle(&ana, &contract, &sandbox).await?;
    Ok(())
}

async fn test_deposit(
    ana: &Account,
    bob: &Account,
    guardian: &Account,
    contract: &Contract,
) -> Result<(), Box<dyn std::error::Error>> {
    let guardian_deposit = guardian
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    let ana_deposit = ana
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(50))
        .max_gas()
        .transact()
        .await?;

    let bob_deposit = bob
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    assert!(guardian_deposit.is_success());
    assert!(ana_deposit.is_success());
    assert!(bob_deposit.is_success());

    let ana_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": ana.id()}))
        .await?
        .json::<UserInfo>()?;
    assert_eq!(ana_balance.staked.as_yoctonear(), NearToken::from_near(50).as_yoctonear());

    let bob_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": bob.id()}))
        .await?
        .json::<UserInfo>()?;
    assert_eq!(bob_balance.staked.as_yoctonear(), NearToken::from_near(1).as_yoctonear());

    let guardian_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": guardian.id()}))
        .await?
        .json::<UserInfo>()?;
    assert_eq!(
        guardian_balance.staked.as_yoctonear(),
        NearToken::from_near(1).as_yoctonear()
    );

    let pool_info = contract.view("get_pool_info").await?.json::<Pool>()?;
    assert_eq!(
        pool_info.tickets.as_yoctonear(),
        ana_balance.staked.saturating_add(bob_balance.staked).saturating_add(guardian_balance.staked).as_yoctonear()
    );

    Ok(())
}

async fn test_raffle(
    ana: &Account,
    contract: &Contract,
    sandbox: &Worker<Sandbox>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Fast forward 200 blocks
    let blocks_to_advance = 200;
    sandbox.fast_forward(blocks_to_advance).await?;

    let prize_update_outcome = ana
        .call(contract.id(), "update_prize")
        .max_gas()
        .transact()
        .await?;

    assert!(prize_update_outcome.is_success());

    let raffle = contract.call("raffle").max_gas().transact().await?;
    let winner = raffle.json::<AccountId>()?;

    assert_eq!(&winner, ana.id());

    Ok(())
}
