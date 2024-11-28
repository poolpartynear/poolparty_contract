use crate::*;
use near_sdk::{near, store::LookupMap, BorshStorageKey, NearToken};

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct Winner(pub AccountId, pub NearToken, pub u64);

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct User {
    pub node: u32,
    pub unstaked: u128,
    pub available_when: u64, // ASK?
    pub withdraw_turn: Option<u64>,
}

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct UserNode {
    pub account_id: AccountId,
    pub weight: u128,
    pub staked: u128,
}

#[near(serializers=[borsh, serde])]
pub struct Users {
    pub map: LookupMap<AccountId, User>,
    pub tree: Vector<UserNode>,
}

#[near(serializers = [borsh])]
#[derive(BorshStorageKey)]
enum StorageKey {
    Users,
    Tree,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            map: LookupMap::new(StorageKey::Users),
            tree: Vector::new(StorageKey::Tree),
        }
    }
}

#[near]
impl Contract {
    pub fn is_registered(&self, user: &AccountId) -> bool {
        self.users.map.contains_key(user)
    }

    pub fn get_user(&self, user: &AccountId) -> &User {
        self.users.map.get(user).expect("User not found!")
    }

    pub fn get_staked_for(&self, user: &AccountId) -> u128 {
        let user = self.get_user(&user);
        let user_node = self.users.tree.get(user.node).expect("User not found!");
        user_node.staked
    }

    pub fn get_withdraw_turn_for(&self, user: &AccountId) -> Option<u64> {
        let user = self.get_user(&user);
        user.withdraw_turn
    }

    pub(crate) fn add_new_user(&mut self, user: &AccountId) -> u32 {
        let uid = self.users.tree.len() as u32;

        self.users.map.insert(
            user.clone(),
            User {
                node: uid,
                unstaked: 0,
                available_when: 0,
                withdraw_turn: None,
            },
        );

        self.users.tree.push(UserNode {
            weight: 0,
            staked: 0,
            account_id: user.clone(),
        });

        return uid;
    }

    pub(crate) fn stake_tickets_for(&mut self, user: &AccountId, tickets: u128) {
        let mut uid = self.users.map[user].node;

        self.users.tree[uid].staked += tickets;
        self.users.tree[uid].weight += tickets;

        while uid != 0 {
            uid = (uid - 1) / 2;
            self.users.tree[uid].weight += tickets;
        }
    }

    pub(crate) fn unstake_tickets_for(&mut self, user: &AccountId, amount: NearToken) {
        self.remove_tickets_from(user, amount.as_yoctonear());

        let current_user = self.users.map.get_mut(user).expect("User not found!");

        current_user.unstaked += amount.as_yoctonear();
    }

    pub(crate) fn withdraw_all_for(&mut self, user: &AccountId) -> u128 {
        let current_user = self.users.map.get_mut(user).expect("User not found!");

        let unstaked_balance = current_user.unstaked;

        current_user.unstaked = 0;

        unstaked_balance
    }

    pub(crate) fn set_withdraw_turn_for(&mut self, user: &AccountId, turn: u64) {
        let user = self.users.map.get_mut(user).expect("User not found!");
        user.withdraw_turn = Some(turn);
    }

    pub(crate) fn set_withdraw_epoch_for(&mut self, user: &AccountId, epoch: u64) {
        let user = self.users.map.get_mut(user).expect("User not found!");
        user.available_when = epoch;
    }

    fn remove_tickets_from(&mut self, user: &AccountId, amount: u128) {
        self.pool
            .tickets
            .saturating_sub(NearToken::from_yoctonear(amount));

        let mut uid = self.users.map[user].node;
        self.users.tree[uid].staked -= amount;
        self.users.tree[uid].weight -= amount;

        while uid != 0 {
            uid = (uid - 1) / 2;
            self.users.tree[uid].weight -= amount;
        }
    }

    pub(crate) fn choose_random_winner(&self) -> AccountId {
        let mut winning_ticket: u128 = 0;

        // accum_weights[0] has the total of tickets in the pool
        // user_staked[0] is the tickets of the pool(guardian)

        if self.users.tree[0].weight > self.users.tree[0].staked {
            winning_ticket = self.random_u128(self.users.tree[0].staked, self.users.tree[0].weight);
        }

        let uid = self.find_user_with_ticket(winning_ticket);

        self.users.tree[uid].account_id.clone()
    }

    fn random_u128(&self, min: u128, max: u128) -> u128 {
        let random_seed = env::random_seed();
        let random = self.as_u128(random_seed.get(..16).unwrap());
        random % (max - min) + min
    }

    // TODO: Consult with Rust proficient
    fn as_u128(&self, arr: &[u8]) -> u128 {
        let mut result: u128 = 0;
        for i in 0..arr.len() {
            result = result * 256 + arr[i] as u128;
        }
        result
    }

    fn find_user_with_ticket(&self, ticket: u128) -> u32 {
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
                winning_ticket =
                    winning_ticket - self.users.tree[uid].staked - self.users.tree[left].weight;
                uid = right
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    #[test]
    fn test_new_contract() {}
}
