use crate::*;
use near_sdk::{json_types::U128, near, store::LookupMap, BorshStorageKey, NearToken};

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct Winner(pub AccountId, pub NearToken, pub u64);

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct UserBalance {
    staked: u128,
    unstaked: u128,
    available_when: u64,
    available: bool,
    withdraw_turn: Option<u64>,
}

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct UserNode {
    weight: u128,
    staked: u128,
    account_id: AccountId,
}

#[near(serializers=[borsh, serde])]
pub struct Users {
    user_map: LookupMap<AccountId, UserBalance>,
    user_to_uid: LookupMap<AccountId, u32>,
    tree: Vector<UserNode>,
}

#[near(serializers = [borsh])]
#[derive(BorshStorageKey)]
enum StorageKey {
    UserToUID,
    Users,
    Tree,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            user_map: LookupMap::new(StorageKey::Users),
            user_to_uid: LookupMap::new(StorageKey::UserToUID),
            tree: Vector::new(StorageKey::Tree),
        }
    }
}

#[near]
impl Contract {
    pub fn is_registered(&self, user: &AccountId) -> bool {
        self.users.user_map.contains_key(user)
    }

    pub fn get_user(&self, user: &AccountId) -> &UserBalance {
        self.users.user_map.get(user).expect("User not found!")
    }

    fn get_user_uid(&self, user: &AccountId) -> &u32 {
        self.users.user_to_uid.get(user).expect("User not found!")
    }

    pub fn get_staked_for(&self, user: &AccountId) -> U128 {
        let user = self.get_user(&user);
        U128(user.staked)
    }

    pub fn get_unstaked_for(&self, user: AccountId) -> U128 {
        let user = self.get_user(&user);
        U128(user.unstaked)
    }

    pub fn get_withdraw_turn_for(&self, user: &AccountId) -> Option<u64> {
        let user = self.get_user(&user);
        user.withdraw_turn
    }

    // Setters
    pub(crate) fn add_new_user(&mut self, user: &AccountId) -> u32 {
        let uid = self.users.tree.len() as u32;

        self.users.user_map.insert(
            user.clone(),
            UserBalance {
                staked: 0,
                unstaked: 0,
                available_when: 0,
                available: false,
                withdraw_turn: None,
            },
        );
        
        self.users.user_to_uid.insert(user.clone(), uid);

        self.users.tree.push(UserNode {
            weight: 0,
            staked: 0,
            account_id: user.clone(),
        });

        return uid;
    }

    pub(crate) fn stake_tickets_for(&mut self, user: &AccountId, tickets: u128) {
        let mut uid = *self.get_user_uid(&user);

        let current_user = self.users.user_map.get_mut(user).expect("User not found!");

        current_user.staked = current_user.staked.saturating_add(tickets);

        self.users.tree[uid].staked += tickets;

        while uid != 0 {
            uid = (uid - 1) / 2;
            self.users.tree[uid].weight += self.users.tree[uid].weight.saturating_add(tickets);
        }
    }

    fn remove_tickets_from(&mut self, user: &AccountId, amount: u128) {
        let mut uid = *self.get_user_uid(user);

        self.users.tree[uid].staked += amount;

        while uid != 0 {
            uid = (uid - 1) / 2;
            self.users.tree[uid].weight -= self.users.tree[uid].weight.saturating_sub(amount);
        }
    }

    pub(crate) fn unstake_tickets_for(&mut self, user: &AccountId, amount: NearToken) {
        self.remove_tickets_from(user, amount.as_yoctonear());

        let current_user = self.users.user_map.get_mut(user).expect("User not found!");

        current_user.staked += amount.as_yoctonear();
        current_user.unstaked += amount.as_yoctonear();
    }

    pub(crate) fn withdraw_all_for(&mut self, user: &AccountId) -> u128 {
        let current_user = self.users.user_map.get_mut(user).expect("User not found!");

        let unstaked_balance = current_user.unstaked;

        current_user.unstaked = 0;

        if current_user.staked == 0 {
            self.users.user_to_uid.remove(user);
            self.users.user_map.remove(user);
        }

        unstaked_balance
    }

    pub(crate) fn set_withdraw_turn_for(&mut self, user: &AccountId, turn: u64) {
        let user = self.users.user_map.get_mut(user).expect("User not found!");

        user.withdraw_turn = Some(turn);
    }

    // TODO: needs to return u182 in range
    // Returns a random number between min (included) and max (excluded)
    // return u128.from(math.randomBuffer(16)) % (max_exc - min_inc) + min_inc
    fn random_u128(&self, min: u128, max: u128) -> u128 {
        let random_seed = env::random_seed(); // TODO: Consider RNG
        let random = self.as_u128(random_seed.get(..16).unwrap());
        random % (max - min) + min
    }

    // TODO: Consult with Rust profficient
    fn as_u128(&self, arr: &[u8]) -> u128 {
        ((arr[0] as u128) << 0)
            + ((arr[1] as u128) << 8)
            + ((arr[2] as u128) << 16)
            + ((arr[3] as u128) << 24)
            + ((arr[4] as u128) << 32)
            + ((arr[5] as u128) << 40)
            + ((arr[6] as u128) << 48)
            + ((arr[7] as u128) << 56)
            + ((arr[8] as u128) << 64)
            + ((arr[9] as u128) << 72)
            + ((arr[10] as u128) << 80)
            + ((arr[11] as u128) << 88)
            + ((arr[12] as u128) << 96)
            + ((arr[13] as u128) << 104)
            + ((arr[14] as u128) << 112)
            + ((arr[15] as u128) << 120)
    }

    pub(crate) fn choose_random_winner(&self) -> AccountId {
        let mut winning_ticket: u128 = 0;

        // accum_weights[0] has the total of tickets in the pool
        // user_staked[0] is the tickets of the pool

        log!("acum weights pool {}, user staked {}", self.users.tree[0].weight, self.users.tree[0].staked);

        if self.users.tree[0].weight > self.users.tree[0].staked {
            winning_ticket = self.random_u128(self.users.tree[0].staked, self.users.tree[0].weight);
        log!("winning ticket {}", winning_ticket);
        }

        let uid = self.find_user_with_ticket(winning_ticket);

        self.users.tree[uid].account_id.clone()
    }

    pub fn find_user_with_ticket(&self, ticket: u128) -> u32 {
        // Gets the user with the winning ticket by searching in the binary tree.
        // This function enumerates the users in pre-order. This does NOT affect
        // the probability of winning, which is nbr_tickets_owned / tickets_total.
        let mut uid: u32 = 0;
        let mut winning_ticket = ticket;

        loop {
            let left: u32 = uid * 2 + 1;
            let right: u32 = uid * 2 + 2;

            if winning_ticket < self.users.tree[uid].staked {
                return uid;
            }

            if winning_ticket < self.users.tree[uid].staked + self.users.tree[left].weight {
                winning_ticket -= self.users.tree[uid].staked;
                uid = left
            } else {
                winning_ticket = winning_ticket - self.users.tree[uid].staked - self.users.tree[uid].staked;
                uid = right
            }
        }
    }
}
