use crate::*;
use near_sdk::{json_types::U128, near, store::LookupMap, BorshStorageKey, NearToken};

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct Winner(pub AccountId, pub NearToken, pub u64);

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct User {
    pub node: u32,
    pub unstaked: u128,
    pub withdraw_turn: Option<u64>,
}

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct UserNode {
    pub account_id: AccountId,
    pub weight: u128,
    pub staked: u128,
}

#[near(serializers = [borsh, serde])]
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

    pub(crate) fn get_withdraw_turn_for(&self, user: &AccountId) -> Option<u64> {
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
        user.withdraw_turn = Some(turn)
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
            winning_ticket = self
                .random_u128(
                    U128(self.users.tree[0].staked),
                    U128(self.users.tree[0].weight),
                )
                .0;
        }

        let uid = self.find_user_with_ticket(winning_ticket);

        self.users.tree[uid].account_id.clone()
    }

    #[private]
    pub fn random_u128(&self, min: U128, max: U128) -> U128 {
        let random_seed = env::random_seed();
        let random = self.as_u128(random_seed.get(..16).unwrap());
        U128(random % (max.0 - min.0) + min.0)
    }

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
    use near_sdk::{testing_env, Gas};

    #[test]
    fn test_user_handling() {
        let guardian: AccountId = "guardian".parse().unwrap();
        let mut contract = Contract::new(
            accounts(0),
            guardian.clone(),
            None,
            None,
            Some(NearToken::from_yoctonear(1)),
            None,
            None,
            None,
        );

        set_context(&guardian, NearToken::from_yoctonear(1));
        contract.deposit_and_stake();

        contract.deposit_and_stake_callback(Ok(()), guardian.clone(), NearToken::from_yoctonear(1));

        for i in 1..3 {
            set_context(
                &format!("user{}", i).parse().unwrap(),
                NearToken::from_yoctonear((i + 1) as u128),
            );

            contract.deposit_and_stake();

            set_context(&"contract".parse().unwrap(), NearToken::from_near(1));

            contract.deposit_and_stake_callback(
                Ok(()),
                format!("user{}", i).parse().unwrap(),
                NearToken::from_yoctonear((1 + i) as u128),
            );
        }

        set_context(&"user1".parse().unwrap(), NearToken::from_yoctonear(0));
        contract.unstake(NearToken::from_yoctonear(1));
        
        assert_eq!(contract.pool.tickets, NearToken::from_yoctonear(6));
        assert_eq!(contract.pool.to_unstake, NearToken::from_yoctonear(1));
    }

    #[test]
    fn test_users_tree() {
        let guardian: AccountId = "guardian".parse().unwrap();
        let mut contract = Contract::new(
            accounts(0),
            guardian.clone(),
            None,
            None,
            Some(NearToken::from_yoctonear(1)),
            None,
            None,
            None,
        );

        set_context(&guardian, NearToken::from_yoctonear(1));
        contract.deposit_and_stake();

        contract.deposit_and_stake_callback(Ok(()), guardian.clone(), NearToken::from_yoctonear(1));

        for i in 1..10 {
            set_context(
                &format!("user{}", i).parse().unwrap(),
                NearToken::from_yoctonear((i + 1) as u128),
            );

            contract.deposit_and_stake();

            set_context(&"contract".parse().unwrap(), NearToken::from_near(1));

            contract.deposit_and_stake_callback(
                Ok(()),
                format!("user{}", i).parse().unwrap(),
                NearToken::from_yoctonear((1 + i) as u128),
            );
        }

        assert!(weights_equal(
            &contract,
            &[55, 38, 16, 21, 15, 6, 7, 8, 9, 10]
        ));

        // Modify participants weights
        set_context(&"contract".parse().unwrap(), NearToken::from_near(1));

        contract.deposit_and_stake_callback(
            Ok(()),
            "user5".parse().unwrap(),
            NearToken::from_yoctonear(2),
        );

        contract.deposit_and_stake_callback(
            Ok(()),
            "user7".parse().unwrap(),
            NearToken::from_yoctonear(1),
        );

        assert!(weights_equal(
            &contract,
            &[58, 39, 18, 22, 15, 8, 7, 9, 9, 10]
        ));

        contract.deposit_and_stake_callback(
            Ok(()),
            "user3".parse().unwrap(),
            NearToken::from_yoctonear(3),
        );

        assert!(weights_equal(
            &contract,
            &[61, 42, 18, 25, 15, 8, 7, 9, 9, 10]
        ));

        contract.deposit_and_stake_callback(Ok(()), guardian.clone(), NearToken::from_yoctonear(1));
        assert!(weights_equal(
            &contract,
            &[62, 42, 18, 25, 15, 8, 7, 9, 9, 10]
        ));

        set_context(&"user8".parse().unwrap(), NearToken::from_yoctonear(0));
        contract.unstake(NearToken::from_yoctonear(1));

        assert!(weights_equal(
            &contract,
            &[61, 41, 18, 24, 15, 8, 7, 9, 8, 10]
        ));

        set_context(&"user4".parse().unwrap(), NearToken::from_yoctonear(0));
        contract.unstake(NearToken::from_yoctonear(3));

        assert!(weights_equal(
            &contract,
            &[58, 38, 18, 24, 12, 8, 7, 9, 8, 10]
        ));

        assert_eq!(contract.find_user_with_ticket(0u128), 0);
        assert_eq!(contract.find_user_with_ticket(1u128), 0);
        assert_eq!(contract.find_user_with_ticket(2u128), 1);
        assert_eq!(contract.find_user_with_ticket(3u128), 1);
        assert_eq!(contract.find_user_with_ticket(40u128), 2);
        assert_eq!(contract.find_user_with_ticket(41u128), 2);
        assert_eq!(contract.find_user_with_ticket(4u128), 3);
        assert_eq!(contract.find_user_with_ticket(9u128), 3);
        assert_eq!(contract.find_user_with_ticket(44u128), 5);
        assert_eq!(contract.find_user_with_ticket(50u128), 5);
        assert_eq!(contract.find_user_with_ticket(51u128), 6);
        assert_eq!(contract.find_user_with_ticket(52u128), 6);
        assert_eq!(contract.find_user_with_ticket(57u128), 6);
        assert_eq!(contract.find_user_with_ticket(11u128), 7);
    }

    fn set_context(account: &AccountId, attached_deposit: NearToken) {
        let context = VMContextBuilder::new()
            .account_balance(NearToken::from_near(20))
            .predecessor_account_id(account.clone())
            .current_account_id("contract".parse().unwrap())
            .attached_deposit(attached_deposit)
            .prepaid_gas(Gas::from_tgas(300))
            .build();

        testing_env!(context);
    }

    fn weights_equal(contract: &Contract, expected_weights: &[u128]) -> bool {
        let weights: Vec<u128> = contract
            .users
            .tree
            .iter()
            .map(|user| user.weight)
            .collect::<Vec<u128>>();

        weights == expected_weights
    }
}
