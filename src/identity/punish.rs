use std::collections::HashMap;

use async_std::task::spawn_blocking;

use crate::{identity::{next_timestamp, proof::MAX_IDT_BY_PROOF}, state::{IdtAmount, ProofId, State, UserAddress}};

// allows to ban for twice the entire balance, i.e. permanent ban. However due to penalty decay
// IDT balance can eventually become positive.
// It only limits vouchee penalty because we do not want to limit amount of penalties and their value
// for a single user but we do not want to propagate it across the network indefinitely.
pub const MAX_VOUCHEE_PENALTY: IdtAmount = MAX_IDT_BY_PROOF * 2;
// vouchee's penalty is multipled to this coefficient before adding to voucher penalty
pub const PENALTY_VOUCHEE_WEIGHT: f64 = 0.1;

#[derive(Clone)]
struct VisitNode {
    pub children_visited: bool,
    // using im::HashSet for better memory usage in set clone operations
    pub visited_branch: im::HashSet<UserAddress>,
}

pub fn punish(
    state: &mut State,
    user: UserAddress,
    moderator: UserAddress,
    penalty: IdtAmount,
    proof_id: ProofId,
) {
    // no balance check on penalty, up to the moderator to decide
    state.punish(user, moderator, penalty, proof_id, next_timestamp());
}

pub async fn penalty_async(state: State, user: UserAddress) -> IdtAmount {
    spawn_blocking(move || penalty(&state, &user)).await
}

fn penalty_from_vouchees(
    state: &State,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    penalties: &HashMap<UserAddress, IdtAmount>,
) -> IdtAmount {
    let mut penalty: IdtAmount = 0;
    for v in &state.vouchees(user) {
        if visited.contains(v) {
            continue;
        }
        // child penalty could be missing due to cyclic dependency.
        let vouchee_penalty = match penalties.get(v) {
            None => continue,
            Some(x) => x,
        };
        let vouchee_penalty_limited = if *vouchee_penalty > MAX_VOUCHEE_PENALTY {
            MAX_VOUCHEE_PENALTY
        } else {
            *vouchee_penalty
        };
        penalty += vouchee_penalty_limited;
    }
    ((penalty as f64) * PENALTY_VOUCHEE_WEIGHT).floor() as IdtAmount
}

pub fn penalty(state: &State, user: &UserAddress) -> IdtAmount {
    // this DFS tree search is a copypaste from IDT balance calculation
    // but it is not worth an effort to refactor it into a common function
    let mut stack = vec![];
    // penalties may have different values for the same user but during branch
    // processing it should have the same penalty for the same user
    let mut penalties: HashMap<UserAddress, IdtAmount> = HashMap::default();

    stack.push((
        user.clone(),
        VisitNode {
            children_visited: false,
            visited_branch: im::HashSet::default(),
        },
    ));

    loop {
        let (last_user, visit_node) = match stack.pop() {
            None => return penalties.get(user).cloned().unwrap_or_default(),
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
            for v in state.vouchees(&last_user) {
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

        let proven_penalty = match state.penalty_event(&last_user) {
            None => 0,
            Some(e) => e.idt_balance,
        };

        let vouchees_penalty = penalty_from_vouchees(
            state,
            &last_user,
            &visit_node.visited_branch,
            &penalties,
        );
        penalties.insert(last_user, proven_penalty + vouchees_penalty);
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::{idt::idt_balance, proof::idt_by_proof, tests::{MODERATOR, PROOF_ID, USER_A}, vouch::vouch};

    use super::*;

    #[test]
    fn test_basic() {
        let mut state = State::default();
        assert!(state.penalty_event(&USER_A.to_string()).is_none());
        punish(&mut state, USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID);
        assert!(state.penalty_event(&USER_A.to_string()).is_some());
        assert_eq!(
            state
                .penalty_event(&USER_A.to_string())
                .unwrap()
                .idt_balance,
            100
        );
        assert_eq!(
            state.penalty_event(&USER_A.to_string()).unwrap().moderator,
            MODERATOR
        );
        assert_eq!(
            state.penalty_event(&USER_A.to_string()).unwrap().proof_id,
            PROOF_ID
        );
        assert!(state.penalty_event(&USER_A.to_string()).unwrap().timestamp > 0);
    }

    #[async_std::test]
    async fn test_penalty() {
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        punish(&mut state, USER_A.to_string(), MODERATOR.to_string(), 50, PROOF_ID);
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 50);
        assert_eq!(penalty(&state, &USER_A.to_string()), 50);
    }

    #[async_std::test]
    async fn test_max_penalty() {
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        punish(&mut state, USER_A.to_string(), MODERATOR.to_string(), 200, PROOF_ID);
        // cannot go lower than 0
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()), 200);
    }

    #[async_std::test]
    async fn test_penalty_for_voucher() {
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
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
        punish(&mut state, user_b.to_string(), MODERATOR.to_string(), 50, PROOF_ID);
        // 100 - 0.1 * 50
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 95);
        assert_eq!(penalty(&state, &USER_A.to_string()), 5);
        // penalty affects vouchee twice:
        // first, from the direct punishment
        // second, from the voucher reduced balance from the vouchee penalty
        // 200 - 50 + 0.1 * 95
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 159);
        assert_eq!(penalty(&state, &user_b.to_string()), 50);
    }

    #[async_std::test]
    async fn test_max_penalty_from_vouchees() {
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
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
        punish(&mut state, user_b.to_string(), MODERATOR.to_string(), 50000, PROOF_ID);
        assert_eq!(penalty(&state, &user_b.to_string()), 50000);
        assert_eq!(penalty(&state, &USER_A.to_string()), 5000);
        // balance is zero due to very high penalty
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 0);
        punish(&mut state, user_b.to_string(), MODERATOR.to_string(), 100000, PROOF_ID);
        assert_eq!(penalty(&state, &user_b.to_string()), 100000);
        assert_eq!(penalty(&state, &USER_A.to_string()), 10000);
        punish(&mut state, user_b.to_string(), MODERATOR.to_string(), 150000, PROOF_ID);
        // penalty for a user is unlimited, but
        // penalty from vouchees is limited to 2 * MAX_IDT_BY_PROOF
        assert_eq!(penalty(&state, &user_b.to_string()), 150000);
        assert_eq!(penalty(&state, &USER_A.to_string()), 10000);
    }
}
