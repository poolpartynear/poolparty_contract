use near_sdk::NearToken;
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
