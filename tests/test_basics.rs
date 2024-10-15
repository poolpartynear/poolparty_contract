use near_sdk::NearToken;
use serde_json::json;

#[tokio::test]
async fn test_contract_is_operational() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let contract_wasm = near_workspaces::compile_project("./").await?;

    let contract = sandbox.dev_deploy(&contract_wasm).await?;
    let staking_contract = sandbox.dev_deploy(&std::fs::read("res/staking.wasm")?).await?;

    let user_account = sandbox.dev_create_account().await?;

    let outcome = user_account
        .call(staking_contract.id(), "deposit_and_stake")
        .args_json(json!({}))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;

    assert!(outcome.is_success());

    // let user_message_outcome = contract.view("get_greeting").args_json(json!({})).await?;
    // assert_eq!(user_message_outcome.json::<String>()?, "Hello World!");

    Ok(())
}
