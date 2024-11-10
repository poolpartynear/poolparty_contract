use near_sdk::NearToken;
use serde_json::json;

use crate::test_env::*;

#[tokio::test]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
