// Find all our documentation at https://docs.near.org
use near_sdk::{
    env, json_types::U64, log, near, store::Vector, AccountId, NearToken, PanicOnDefault,
};
use pool::Pool;
use users::Users;

pub const NO_ARGS: Vec<u8> = vec![];
pub const NO_DEPOSIT: NearToken = NearToken::from_near(0);

// The raffle happens once per day (expressed in ms)
const RAFFLE_WAIT: U64 = U64(86400000);

// The users cannot have more than a certain amount of NEARs,
// to limit whale's size in the pool. Default: A thousand NEARs
const MAX_DEPOSIT: NearToken = NearToken::from_near(1000);

// The users cannot have deposit less than a certain amount of
// NEARs, to limit sybill attacks. Default: 1 NEAR
const MIN_DEPOSIT: NearToken = NearToken::from_near(1);

// Amount of epochs to wait before unstaking
const EPOCHS_WAIT: u64 = 4;

// Minimum amount to Raffle (0.1 NEAR)
const MIN_TO_RAFFLE: NearToken = NearToken::from_millinear(100);

// Maximum amount to Raffle (50 NEAR)
const MAX_TO_RAFFLE: NearToken = NearToken::from_near(100);

pub mod external;
pub mod pool;
pub mod users;

#[near(serializers=[borsh])]
pub enum Action {
    Unstake,
    Withdraw,
}

#[near(serializers=[json])]
#[derive(Clone)]
pub struct UserInfo {
    pub staked: NearToken,
    pub unstaked: NearToken,
    pub available: bool,
    pub withdraw_turn: u8,
}

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct Config {
    external_pool: AccountId,
    min_to_raffle: NearToken,
    max_to_raffle: NearToken,
    min_deposit: NearToken,
    max_deposit: NearToken,
    epochs_wait: u64,
    time_between_raffles: u64,
    guardian: AccountId,
    pub emergency: bool,
}

// Define the contract structure
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: Config,
    pool: Pool,
    users: Users,
    next_action: Action,
}

// Implement the contract structure
#[near]
impl Contract {
    #[private]
    #[init]
    pub fn new(
        external_pool: AccountId,
        guardian: AccountId,
        first_raffle: U64,
        min_to_raffle: Option<NearToken>,
        max_to_raffle: Option<NearToken>,
        min_deposit: Option<NearToken>,
        max_deposit: Option<NearToken>,
        epochs_wait: Option<u64>,
        time_between_raffles: Option<U64>,
    ) -> Self {
        Self {
            config: Config {
                external_pool,
                guardian,
                max_to_raffle: max_to_raffle.unwrap_or(MAX_TO_RAFFLE),
                min_to_raffle: min_to_raffle.unwrap_or(MIN_TO_RAFFLE),
                min_deposit: min_deposit.unwrap_or(MIN_DEPOSIT),
                max_deposit: max_deposit.unwrap_or(MAX_DEPOSIT),
                epochs_wait: epochs_wait.unwrap_or(EPOCHS_WAIT),
                time_between_raffles: time_between_raffles.unwrap_or(RAFFLE_WAIT).0,
                emergency: false,
            },
            pool: Pool::new(first_raffle.0),
            users: Users::default(),
            next_action: Action::Unstake,
        }
    }

    pub fn get_config(&self) -> Config {
        self.config.clone()
    }

// export function get_account(account_id: string): Users.User {
  // Returns information for the account 'account_id'
//   if (!Users.is_registered(account_id)) {
//     return new Users.User(u128.Zero, u128.Zero, 0, false)
//   }

//   const tickets: u128 = Users.get_staked_for(account_id)
//   const unstaked: u128 = Users.get_unstaked_for(account_id)

//   const when: u64 = Users.get_withdraw_turn_for(account_id)
//   const now: u64 = External.get_current_turn()

//   // Compute remaining time for withdraw to be ready
//   const remaining: u64 = (when > now) ? when - now : 0

//   const available: bool = unstaked > u128.Zero && now >= when

//   return new Users.User(tickets, unstaked, remaining, available)
// }

    pub fn get_user_info(&self, user: AccountId) -> UserInfo {
        
        match self.users.map.get(&user) {
            Some(user) => {
                let uid = user.node;
                let user_node = self.users.tree[uid].clone(); 
                
                let staked = user_node.staked;
                let when = user.withdraw_turn.unwrap_or(0);
                let now = self.pool.next_withdraw_epoch - 1;

                let remaining = if when > now { when - now } else { 0 };
                let available = user.unstaked > 0 && now >= when;

                UserInfo {
                    staked: NearToken::from_yoctonear(staked),
                    unstaked: NearToken::from_yoctonear(user.unstaked),
                    available,
                    withdraw_turn: remaining as u8,
                }
            },
            None => {
                UserInfo {
                    staked: NearToken::from_yoctonear(0),
                    unstaked: NearToken::from_yoctonear(0),
                    available: false,
                    withdraw_turn: 0,
                }
            }
        }
    }

    #[private]
    pub fn emergency_stop(&mut self) {
        self.config.emergency = false;
    }

    #[private]
    pub fn emergency_start(&mut self) {
        self.config.emergency = true;
    }

    #[private]
    pub fn set_time_between_raffles(&mut self, time: U64) {
        self.config.time_between_raffles = time.0;
    }

    #[private]
    pub fn set_epochs_wait(&mut self, epochs: u64) {
        self.config.epochs_wait = epochs;
    }
}
