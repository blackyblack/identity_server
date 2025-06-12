use std::collections::HashMap;

use crate::{identity::punish::penalty_async, state::{IdtAmount, State, UserAddress}};

pub const TOP_VOUCHERS_SIZE: u16 = 5;
// voucher's balance is multipled to this coefficient before adding to vouchee balance
pub const VOUCHER_WEIGHT: f64 = 0.1;

#[derive(Clone)]
struct VisitNode {
    pub children_visited: bool,
    // using im::HashSet for better memory usage in set clone operations
    // im_rc cannot be used here because Tide state requires Send + Sync
    pub visited_branch: im::HashSet<UserAddress>,
}

fn top_vouchers(
    state: &State,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    balances: &HashMap<UserAddress, IdtAmount>,
) -> Vec<(UserAddress, IdtAmount)> {
    let mut top_balances: Vec<(UserAddress, IdtAmount)> = vec![];
    for v in &state.vouchers(user) {
        if visited.contains(v) {
            continue;
        }
        // child balance could be missing due to cyclic dependency.
        let voucher_balance = match balances.get(v) {
            None => continue,
            Some(x) => x,
        };
        top_balances.push((v.clone(), *voucher_balance));
    }
    top_balances.sort();
    top_balances.reverse();
    top_balances.truncate(TOP_VOUCHERS_SIZE.into());
    top_balances
}

pub async fn idt_balance(state: &State, user: &UserAddress) -> IdtAmount {
    let mut stack = vec![];
    // balances may have different values for the same user but during branch
    // processing it should have the same balance for the same user
    let mut balances: HashMap<UserAddress, IdtAmount> = HashMap::default();

    stack.push((
        user.clone(),
        VisitNode {
            children_visited: false,
            visited_branch: im::HashSet::default(),
        },
    ));

    loop {
        let (last_user, visit_node) = match stack.pop() {
            None => return balances.get(user).cloned().unwrap_or_default(),
            Some(x) => x,
        };
        if !visit_node.children_visited {
            let mut visited_branch = visit_node.visited_branch;
            visited_branch.insert(last_user.clone());
            stack.push((
                last_user.clone(),
                VisitNode {
                    children_visited: true,
                    // each node has own visited branch because we do not want
                    // other branches to affect balance calculation of the current
                    // branch
                    visited_branch: visited_branch.clone(),
                },
            ));
            for v in state.vouchers(&last_user) {
                if visited_branch.contains(&v) {
                    continue;
                }
                stack.push((
                    v.clone(),
                    VisitNode {
                        children_visited: false,
                        visited_branch: visited_branch.clone(),
                    },
                ));
            }
            continue;
        }

        let proven_balance = match state.proof_event(&last_user) {
            None => 0,
            Some(e) => e.idt_balance,
        };

        let top_vouchers = top_vouchers(state, &last_user, &visit_node.visited_branch, &balances);
        let balance_from_vouchers = top_vouchers.into_iter().fold(0, |acc, (_user, b)| {
            acc + (((b as f64) * VOUCHER_WEIGHT).floor() as u64)
        });
        let penalty = penalty_async(state.clone(), last_user.clone()).await;
        let user_balance = {
            let positive_balance = proven_balance + (balance_from_vouchers as IdtAmount);
            if positive_balance > penalty {
                positive_balance - penalty
            } else {
                0
            }
        };
        balances.insert(last_user, user_balance);
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        proof::idt_by_proof,
        tests::{MODERATOR, PROOF_ID, USER_A},
        vouch::vouch,
    };

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        // IDT of A does not change after vouching
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        // IDT of B increased
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 10);
    }

    #[async_std::test]
    async fn test_cyclic() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 10);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // cyclic vouch does not change user A balance
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 10);
    }

    #[async_std::test]
    async fn test_mutual() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 200);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        // 200 + 0.1 * 100
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // 100 + 0.1 * 200
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 120);
        // not increased due to cyclic dependency
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
    }

    #[async_std::test]
    async fn test_branches() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            20000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_c.to_string(),
            MODERATOR.to_string(),
            30000,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 20000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 30000);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        vouch(&mut state, USER_A.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 21000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 31000);
        vouch(&mut state, user_b.to_string(), user_d.to_string());
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 2100);
        vouch(&mut state, user_c.to_string(), user_d.to_string());
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 5200);
        vouch(&mut state, user_b.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 33100);
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 5410);
    }

    #[async_std::test]
    async fn test_max_vouchers() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let user_e = "userE";
        let user_f = "userF";
        let user_g = "userG";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            2000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_c.to_string(),
            MODERATOR.to_string(),
            3000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_d.to_string(),
            MODERATOR.to_string(),
            4000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_e.to_string(),
            MODERATOR.to_string(),
            5000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_f.to_string(),
            MODERATOR.to_string(),
            6000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_g.to_string(),
            MODERATOR.to_string(),
            7000,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 2000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 3000);
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 4000);
        assert_eq!(idt_balance(&state, &user_e.to_string()).await, 5000);
        assert_eq!(idt_balance(&state, &user_f.to_string()).await, 6000);
        assert_eq!(idt_balance(&state, &user_g.to_string()).await, 7000);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // 1000 + 0.1 * 2000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1200);
        vouch(&mut state, user_c.to_string(), USER_A.to_string());
        // 1200 + 0.1 * 3000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1500);
        vouch(&mut state, user_d.to_string(), USER_A.to_string());
        // 1500 + 0.1 * 4000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1900);
        vouch(&mut state, user_e.to_string(), USER_A.to_string());
        // 1900 + 0.1 * 5000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 2400);
        vouch(&mut state, user_f.to_string(), USER_A.to_string());
        // 2400 + 0.1 * 6000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 3000);
        vouch(&mut state, user_g.to_string(), USER_A.to_string());
        // 3000 + 0.1 * 7000 - 0.1 * 2000
        // only 5 top vouchers are considered
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 3500);
    }

    #[async_std::test]
    async fn test_async() {
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
    }
}
