use near_sdk::NearToken;
use serde_json::json;

use crate::test_env::*;

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

    // println!();
    // println!("{:#?}", deposit_outcome.logs());

    assert!(deposit_outcome.is_success());

    let get_staked: serde_json::Value = alice
        .view(contract.id(), "get_staked_for")
        .args_json(json!({"account_id": alice.id()}))
        .await?
        .json()?;

    println!("{:#?}", get_staked);
    // assert_eq!(get_staked["tickets"].as_u64().unwrap(), 1);

    Ok(())
}
