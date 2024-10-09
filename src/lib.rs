// Find all our documentation at https://docs.near.org
use near_sdk::{
    env, json_types::U64, log, near, store::Vector, AccountId, NearToken, PanicOnDefault,
};
use users::{UserStorage, Winner};

// The raffle happens once per day
const RAFFLE_WAIT: U64 = U64(86400000000000);

// If the tree gets too high (>13 levels) traversing it gets expensive,
// lets cap the max number of users, so traversing the tree is at max 90TGAS
// const MAX_USERS: i32 = 8191;

// The users cannot have more than a certain amount of NEARs,
// to limit whale's size in the pool. Default: A thousand NEARs
const MAX_DEPOSIT: NearToken = NearToken::from_near(1000);

// The users cannot have deposit less than a certain amount of
// NEARs, to limit sybill attacks. Default: 1 NEAR
const MIN_DEPOSIT: NearToken = NearToken::from_near(1);

// Amount of epochs to wait before unstaking (changed for testing)
const EPOCHS_WAIT: U64 = U64(4);

// Minimum amount to Raffle (0.1 NEAR)
const MIN_TO_RAFFLE: NearToken = NearToken::from_millinear(1);

// Maximum amount to Raffle (50 NEAR)
const MAX_TO_RAFFLE: NearToken = NearToken::from_near(50);

pub mod pool;
pub mod users;

#[near(serializers=[borsh, serde])]
#[derive(Clone)]
pub struct Pool {
    pub total_staked: NearToken,
    pub waiting_to_unstake: NearToken,
    pub reserve: NearToken,
    pub prize_pool: NearToken,
    pub last_prize_update: u64,
    pub next_raffle: u64,
    pub withdraw_ready: bool,
    pub pool_tickets: NearToken,
    pub total_users: u64,
    pub winners: Vec<Winner>,
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            total_staked: NearToken::from_yoctonear(0),
            waiting_to_unstake: NearToken::from_yoctonear(0),
            reserve: NearToken::from_yoctonear(0),
            prize_pool: NearToken::from_yoctonear(0),
            last_prize_update: 0,
            next_raffle: 0,
            withdraw_ready: false,
            pool_tickets: NearToken::from_yoctonear(0),
            total_users: 0,
            winners: vec![],
        }
    }
}

#[near(serializers=[borsh, serde])]
#[derive(Clone)]
pub struct Config {
    external_pool: AccountId,
    next_raffle_timestamp: u64,
    min_to_raffle: NearToken,
    max_to_raffle: NearToken,
    min_deposit: NearToken,
    max_deposit: NearToken,
    epochs_wait: u64,
    time_between_raffles: u64,
    emergency: bool,
}

// Define the contract structure
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: Config,
    pool: Pool,
    user_storage: UserStorage,
}

// Implement the contract structure
#[near]
impl Contract {
    #[private]
    #[init]
    pub fn new(
        external_pool: AccountId,
        first_raffle: U64,
        min_to_raffle: Option<NearToken>,
        max_to_raffle: Option<NearToken>,
        min_deposit: Option<NearToken>,
        max_deposit: Option<NearToken>,
        epochs_wait: Option<U64>,
        time_between_raffles: Option<U64>,
    ) -> Self {
        Self {
            config: Config {
                external_pool,
                next_raffle_timestamp: first_raffle.0,
                max_to_raffle: max_to_raffle.unwrap_or(MAX_TO_RAFFLE),
                min_to_raffle: min_to_raffle.unwrap_or(MIN_TO_RAFFLE),
                min_deposit: min_deposit.unwrap_or(MIN_DEPOSIT),
                max_deposit: max_deposit.unwrap_or(MAX_DEPOSIT),
                epochs_wait: epochs_wait.unwrap_or(EPOCHS_WAIT).0,
                time_between_raffles: time_between_raffles.unwrap_or(RAFFLE_WAIT).0,
                emergency: false,
            },
            pool: Pool::default(),
            user_storage: UserStorage::default(),
        }
    }

    // Getters
    pub fn get_tickets(&self) -> NearToken {
        self.pool.pool_tickets
    }

    pub fn get_time_between_raffles(&self) -> U64 {
        U64(self.config.time_between_raffles)
    }

    pub fn get_config(&self) -> Config {
        self.config.clone()
    }

    // Setters
    #[private]
    pub fn change_time_between_raffles(&mut self, time: U64) {
        self.config.time_between_raffles = time.0;
    }

    #[private]
    pub fn change_max_deposit(&mut self, amount: NearToken) {
        self.config.max_deposit = amount;
    }

    #[private]
    pub fn change_min_deposit(&mut self, amount: NearToken) {
        self.config.min_deposit = amount;
    }

    #[private]
    pub fn change_min_raffle(&mut self, amount: NearToken) {
        self.config.min_to_raffle = amount;
    }

    #[private]
    pub fn change_max_raffle(&mut self, amount: NearToken) {
        self.config.max_to_raffle = amount;
    }

    #[private]
    pub fn change_epoch_wait(&mut self, epochs: U64) {
        self.config.epochs_wait = epochs.0;
    }

    #[private]
    pub fn emergency_stop(&mut self) {
        self.config.emergency = false;
    }

    #[private]
    pub fn emergency_start(&mut self) {
        self.config.emergency = true;
    }
}

/*
 * The rest of this file holds the inline tests for the code above
 * Learn more about Rust tests: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
 */
#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn get_default_greeting() {
    //     let contract = Contract::default();
    //     // this test did not call set_greeting so should return the default "Hello" greeting
    //     assert_eq!(contract.get_greeting(), "Hello");
    // }

    // #[test]
    // fn set_then_get_greeting() {
    //     let mut contract = Contract::default();
    //     contract.set_greeting("howdy".to_string());
    //     assert_eq!(contract.get_greeting(), "howdy");
    // }
}
