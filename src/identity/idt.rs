use std::collections::HashMap;

use async_std::task::spawn_blocking;

use crate::state::{IdtAmount, State, UserAddress};

pub const TOP_VOUCHERS_SIZE: u16 = 5;
// voucher's balance is multipled to this coefficient before adding to vouchee balance
pub const VOUCHER_WEIGHT: f64 = 0.1;

#[derive(Clone)]
struct VisitNode {
    pub children_visited: bool,
    // using im_rc::HashSet for better memory usage in set clone operations
    pub visited_branch: im_rc::HashSet<UserAddress>,
}

pub async fn idt_balance_async(state: State, user: UserAddress) -> IdtAmount {
    spawn_blocking(move || idt_balance(&state, &user)).await
}

pub fn idt_balance(state: &State, user: &UserAddress) -> IdtAmount {
    let mut stack = vec![];
    // node_balances may have different values for the same user but during branch
    // processing it should have the same balance for the same user
    let mut node_balances: HashMap<UserAddress, IdtAmount> = HashMap::default();

    stack.push((
        user.clone(),
        VisitNode {
            children_visited: false,
            visited_branch: im_rc::HashSet::default(),
        },
    ));

    loop {
        let (last_user, visit_node) = match stack.pop() {
            None => return node_balances.get(user).cloned().unwrap_or_default(),
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

        let mut top_balances: Vec<IdtAmount> = vec![];
        for v in &state.vouchers(&last_user) {
            if visit_node.visited_branch.contains(v) {
                continue;
            }
            // child balance could be missing due to cyclic dependency.
            let voucher_balance = match node_balances.get(v) {
                None => continue,
                Some(x) => x,
            };
            top_balances.push(*voucher_balance);
        }
        top_balances.sort();
        top_balances.reverse();
        top_balances.truncate(TOP_VOUCHERS_SIZE.into());
        let balance_from_vouchers = top_balances.into_iter().fold(0, |acc, b| {
            acc + (((b as f64) * VOUCHER_WEIGHT).floor() as u64)
        });
        let last_user_balance = proven_balance + (balance_from_vouchers as IdtAmount);
        node_balances.insert(last_user, last_user_balance);
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::{proof::idt_by_proof, tests::{MODERATOR, PROOF_ID, USER_A}, vouch::vouch};

    use super::*;

    #[test]
    fn test_basic() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID);
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()), 0);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        // IDT of A does not change after vouching
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 100);
        // IDT of B increased
        assert_eq!(idt_balance(&state, &user_b.to_string()), 10);
    }

    #[test]
    fn test_cyclic() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()), 10);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // cyclic vouch does not change user A balance
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()), 10);
    }

    #[test]
    fn test_mutual() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_b.to_string(), MODERATOR.to_string(), 200, PROOF_ID);
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()), 200);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 100);
        // 200 + 0.1 * 100
        assert_eq!(idt_balance(&state, &user_b.to_string()), 210);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // 100 + 0.1 * 200
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 120);
        // not increased due to cyclic dependency
        assert_eq!(idt_balance(&state, &user_b.to_string()), 210);
    }

    #[test]
    fn test_branches() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let mut state = State::default();
        let _ = idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 10000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_b.to_string(), MODERATOR.to_string(), 20000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_c.to_string(), MODERATOR.to_string(), 30000, PROOF_ID);
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()), 20000);
        assert_eq!(idt_balance(&state, &user_c.to_string()), 30000);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        vouch(&mut state, USER_A.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &user_b.to_string()), 21000);
        assert_eq!(idt_balance(&state, &user_c.to_string()), 31000);
        vouch(&mut state, user_b.to_string(), user_d.to_string());
        assert_eq!(idt_balance(&state, &user_d.to_string()), 2100);
        vouch(&mut state, user_c.to_string(), user_d.to_string());
        assert_eq!(idt_balance(&state, &user_d.to_string()), 5200);
        vouch(&mut state, user_b.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &user_c.to_string()), 33100);
        assert_eq!(idt_balance(&state, &user_d.to_string()), 5410);
    }

    #[test]
    fn test_max_vouchers() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let user_e = "userE";
        let user_f = "userF";
        let user_g = "userG";
        let mut state = State::default();
        let _ = idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 1000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_b.to_string(), MODERATOR.to_string(), 2000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_c.to_string(), MODERATOR.to_string(), 3000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_d.to_string(), MODERATOR.to_string(), 4000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_e.to_string(), MODERATOR.to_string(), 5000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_f.to_string(), MODERATOR.to_string(), 6000, PROOF_ID);
        let _ = idt_by_proof(&mut state, user_g.to_string(), MODERATOR.to_string(), 7000, PROOF_ID);
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 1000);
        assert_eq!(idt_balance(&state, &user_b.to_string()), 2000);
        assert_eq!(idt_balance(&state, &user_c.to_string()), 3000);
        assert_eq!(idt_balance(&state, &user_d.to_string()), 4000);
        assert_eq!(idt_balance(&state, &user_e.to_string()), 5000);
        assert_eq!(idt_balance(&state, &user_f.to_string()), 6000);
        assert_eq!(idt_balance(&state, &user_g.to_string()), 7000);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // 1000 + 0.1 * 2000
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 1200);
        vouch(&mut state, user_c.to_string(), USER_A.to_string());
        // 1200 + 0.1 * 3000
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 1500);
        vouch(&mut state, user_d.to_string(), USER_A.to_string());
        // 1500 + 0.1 * 4000
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 1900);
        vouch(&mut state, user_e.to_string(), USER_A.to_string());
        // 1900 + 0.1 * 5000
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 2400);
        vouch(&mut state, user_f.to_string(), USER_A.to_string());
        // 2400 + 0.1 * 6000
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 3000);
        vouch(&mut state, user_g.to_string(), USER_A.to_string());
        // 3000 + 0.1 * 7000 - 0.1 * 2000
        // only 5 top vouchers are considered
        assert_eq!(idt_balance(&state, &USER_A.to_string()), 3500);
    }

    #[async_std::test]
    async fn test_async() {
        let mut state = State::default();
        let _ = idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID);
        assert_eq!(idt_balance_async(state, USER_A.to_string()).await, 100);
    }
}
