use crate::*;
use near_sdk::{near, store::LookupMap, BorshStorageKey, NearToken};

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct Winner(pub AccountId, pub NearToken, pub u64);

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct User {
    uid: u32,
    staked_balance: NearToken,
    unstaked_balance: NearToken,
    available_when: u64,
    available: bool,
}

#[near(serializers=[borsh, serde])]
pub struct UserStorage {
    user_to_uid: LookupMap<AccountId, u32>,
    uid_to_user: LookupMap<u32, AccountId>,
    users: LookupMap<AccountId, User>,
    accum_weights: Vector<NearToken>,
    user_staked: Vector<NearToken>,
    total_users: u32,
    total_staked: NearToken,
    total_weight: NearToken,
}

#[near(serializers = [borsh])]
#[derive(BorshStorageKey)]
enum StorageKey {
    UserToUid,
    UidToUser,
    Users,
    AccumWeights,
    UserStaked,
}

impl Default for UserStorage {
    fn default() -> Self {
        Self {
            user_to_uid: LookupMap::new(StorageKey::UserToUid),
            uid_to_user: LookupMap::new(StorageKey::UidToUser),
            users: LookupMap::new(StorageKey::Users),
            accum_weights: Vector::new(StorageKey::AccumWeights),
            user_staked: Vector::new(StorageKey::UserStaked),
            total_users: 0,
            total_staked: NearToken::from_yoctonear(0),
            total_weight: NearToken::from_yoctonear(0),
        }
    }
}

impl UserStorage {
    pub fn is_registered(&self, user: &AccountId) -> bool {
        self.users.contains_key(user)
    }

    pub fn get_user(&self, user: &AccountId) -> &User {
        self.users.get(user).expect("User not found!")
    }

    fn get_user_uid(&self, user: &AccountId) -> u32 {
        let user = self.users.get(user).expect("User not found!");
        user.uid
    }

    pub fn get_staked_for(&self, user: &AccountId) -> NearToken {
        let user = self.users.get(user).expect("User not found!");
        user.staked_balance
    }

    pub fn get_unstaked_for(&self, user: AccountId) -> NearToken {
        let user = self.users.get(&user).expect("User not found!");
        user.unstaked_balance
    }

    // export function get_withdraw_turn_for(user: string): u64 {
    //   const uid: i32 = get_user_uid(user)
    //   return user_withdraw_turn[uid]
    // }
    //

    pub fn get_total_users(&self) -> u32 {
        self.total_users
    }

    // Setters
    fn set_withdraw_turn(&mut self, user: &AccountId) {}

    pub(crate) fn add_new_user(&mut self, user: &AccountId) -> u32 {
        let uid = self.total_users;

        self.users.insert(
            user.clone(),
            User {
                uid,
                staked_balance: NearToken::from_yoctonear(0),
                unstaked_balance: NearToken::from_yoctonear(0),
                available_when: 0,
                available: false,
            },
        );

        self.total_users += 1;

        //     user_unstaked.push(u128.Zero)
        //     user_withdraw_turn.push(0)
        //     accum_weights.push(u128.Zero)
        //     user_staked.push(u128.Zero)
        //   }

          self.user_to_uid.set(user.clone(), uid);
          self.uid_to_user.set(uid, user);

        return uid;
    }

    pub(crate) fn stake_tickets_for(&mut self, user: &AccountId, tickets: NearToken) {
        let uid = self.get_user_uid(&user);

        let current_user = self.users.get_mut(user).expect("User not found!");

        current_user.staked_balance = current_user.staked_balance.saturating_add(tickets);

        // accum_weights[uid] = accum_weights[uid] + amount

        // while (uid != 0) {
        //   uid = (uid - 1) / 2
        //   accum_weights[uid] = accum_weights[uid] + amount
        // }
    }

    pub(crate) fn remove_tickets_from(&mut self, user: &AccountId, amount: NearToken) {}

    fn unstake_tickets_for(&mut self, user: &AccountId, amount: NearToken) {
        self.remove_tickets_from(user, amount);
    }

    pub(crate) fn withdraw_all_for(&mut self, user: &AccountId) -> NearToken {
        let current_user = self.users.get_mut(user).expect("User not found!");

        let unstaked_balance = current_user.unstaked_balance;

        current_user.unstaked_balance = NearToken::from_near(0);

        if current_user.staked_balance.is_zero() {
            self.user_to_uid.remove(user);
            self.users.remove(user);
        }

        unstaked_balance
    }

    // pub(crate) fn set_withdraw_turn_for(&mut self, user: &AccountId) -> {}

    // TODO: needs to return u182 in range
    // Returns a random number between min (included) and max (excluded)
    //   return u128.from(math.randomBuffer(16)) % (max_exc - min_inc) + min_inc
    fn random_u128(&self) -> u128 {
        let random_seed = env::random_seed(); // TODO: Consider RNG
        self.as_u128(random_seed.get(..16).unwrap())
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
        //         let winning_ticket: u128 = u128.Zero

        //   // accum_weights[0] has the total of tickets in the pool
        //   // user_staked[0] is the tickets of the pool

        //   if (accum_weights[0] > user_staked[0]) {
        //     // There are more tickets in the pool than just the reserve
        //     // Choose a winner excluding the reserve. i.e. Exclude range [0, user_staked[0])
        //     winning_ticket = random_u128(user_staked[0], accum_weights[0])
        //   }

        //   const uid: i32 = find_user_with_ticket(winning_ticket)
        //   return uid_to_user.getSome(uid)
    }

    pub fn find_user_with_ticket(&self, winning_ticket: u128) -> u32 {}
    // Gets the user with the winning ticket by searching in the binary tree.
    // This function enumerates the users in pre-order. This does NOT affect
    // the probability of winning, which is nbr_tickets_owned / tickets_total.
    //   let uid: i32 = 0

    //   while (true) {
    //     let left: i32 = uid * 2 + 1;
    //     let right: i32 = uid * 2 + 2;

    //     if (winning_ticket < user_staked[uid]) {
    //       return uid
    //     }

    //     if (winning_ticket < user_staked[uid] + accum_weights[left]) {
    //       winning_ticket = winning_ticket - user_staked[uid]
    //       uid = left
    //     } else {
    //       winning_ticket = winning_ticket - user_staked[uid] - accum_weights[left]
    //       uid = right
    //     }
    //   }
    // }
}
