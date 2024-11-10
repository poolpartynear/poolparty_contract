// use near_sdk::NearToken;
// use near_workspaces::{Account, Contract, DevNetwork, Worker};
// use serde_json::json;

// #[tokio::test]
// async fn contract_test() -> Result<(), Box<dyn std::error::Error>> {
//     let worker = near_workspaces::sandbox().await?;
//     let (contract, staking_contract, _alice, _bob) = init(&worker).await?;

//     let get_config: serde_json::Value = contract
//         .call("get_config")
//         .args_json(json!({}))
//         .view()
//         .await?
//         .json()?;

//     assert!(get_config["external_pool"].as_str().unwrap() == staking_contract.id().as_str());

//     Ok(())
// }

// #[tokio::test]
// async fn adds_tickets_to_user() -> Result<(), Box<dyn std::error::Error>> {
//     let worker = near_workspaces::sandbox().await?;
//     let (contract, _staking_contract, alice, _bob) = init(&worker).await?;

//     let deposit_outcome = alice
//         .call(contract.id(), "deposit_and_stake")
//         .args_json(json!({}))
//         .deposit(NearToken::from_near(2))
//         .max_gas()
//         .transact()
//         .await?;

//     // println!();
//     // println!("{:#?}", deposit_outcome.logs());

//     assert!(deposit_outcome.is_success());

//     // let get_staked: serde_json::Value = alice
//     //     .view(contract.id(), "get_staked_for")
//     //     .args_json(json!({"account_id": alice.id()}))
//     //     .await?
//     //     .json()?;

//     // assert_eq!(get_staked["tickets"].as_u64().unwrap(), 1);

//     Ok(())
// }
