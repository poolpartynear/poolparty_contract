#! /bin/bash
shopt -s expand_aliases
alias near="npx near-cli@latest"  

# Delete Accounts
near delete-account --force lp.flmel.testnet flmel.testnet
near delete-account --force pp2.flmel.testnet flmel.testnet

# Create Accounts
near create-account lp.flmel.testnet --masterAccount flmel.testnet --initialBalance 10
near create-account pp2.flmel.testnet --masterAccount flmel.testnet --initialBalance 10

# Build poolparty Cotnract
cargo near build --no-docker

# Deploy poolparty and mock stakign contracts
near deploy lp.flmel.testnet ./res/staking.wasm
near deploy pp2.flmel.testnet ./target/near/poolparty.wasm    

# Init and stake form 2 users
near call pp2.flmel.testnet new '{"external_pool": "lp.flmel.testnet"}' --useAccount pp2.flmel.testnet
near call pp2.flmel.testnet deposit_and_stake --deposit 3 --useAccount awesomenear.testnet --gas 300000000000000
near call pp2.flmel.testnet deposit_and_stake --deposit 100 --useAccount flmel.testnet --gas 300000000000000

near call pp2.flmel.testnet update_prize --useAccount flmel.testnet --gas 300000000000000
near call pp2.flmel.testnet raffle --useAccount flmel.testnet --gas 300000000000000
