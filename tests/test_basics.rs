use chrono::Utc;
use near_workspaces::{network::Sandbox, types::NearToken, Account, Contract, Worker};
use serde_json::json;

#[tokio::test]
async fn test_contract_is_operational() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let contract_wasm = near_workspaces::compile_project("./").await?;

    let contract = sandbox.dev_deploy(&contract_wasm).await?;
    let staking_contract = sandbox
        .dev_deploy(&std::fs::read("res/staking.wasm")?)
        .await?;

    let now = Utc::now().timestamp();
    let one_minute = 60 * 1000000000;
    let a_minute_from_now = now * 1000000000 + one_minute;

    let init = contract
        .call("new")
        .args_json(json!(
            {
                "external_pool": staking_contract.id(),
                "first_raffle": a_minute_from_now.to_string(),
                "time_between_raffles": one_minute.to_string(),
            }
        ))
        .transact()
        .await?;

    assert!(init.is_success());
        
    let ana = sandbox.dev_create_account().await?;
    let bob = sandbox.dev_create_account().await?;

    // Begin tests
    test_raffle(&ana, &bob, &contract, &sandbox).await?;
    Ok(())
}

async fn test_raffle(
    ana: &Account,
    bob: &Account,
    contract: &Contract,
    sandbox: &Worker<Sandbox>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = ana
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    let _ = bob
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;

    // Fast forward 200 blocks
    let blocks_to_advance = 200;
    sandbox.fast_forward(blocks_to_advance).await?;

    let _ = contract
        .call("raffle")
        .max_gas()
        .transact()
        .await?;

    let winners = contract
        .view("get_winners")
        .await?
        .json::<Vec::<String>>()?;

    assert_eq!(winners[0], ana.id().to_string());

    Ok(())
}
