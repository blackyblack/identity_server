use std::collections::HashMap;

use crate::identity::{IdtAmount, UserAddress, error::Error};

pub trait Visitor {
    // called when all children of the node are processed
    // visited_branch contains all children, no duplicates
    // balances contain temporary balances of the processed nodes
    // returns balance of the current node
    async fn exit_node(
        &self,
        node: &UserAddress,
        visited_branch: &im::HashSet<UserAddress>,
        balances: &HashMap<UserAddress, IdtAmount>,
    ) -> Result<IdtAmount, Error>;
}

pub trait ChildrenSelector {
    async fn children(&self, root: &UserAddress) -> Result<Vec<UserAddress>, Error>;
}

#[derive(Clone)]
struct VisitNode {
    pub children_visited: bool,
    // using im::HashSet for better memory usage in set clone operations
    pub visited_branch: im::HashSet<UserAddress>,
}

pub async fn walk_tree<T>(tree: &T, root: &UserAddress) -> Result<IdtAmount, Error>
where
    T: ChildrenSelector + Visitor,
{
    // Stack used for depth-first traversal of the tree
    let mut stack = vec![];
    // balances may have different values for the same user but during branch
    // processing it should have the same balance for the same user
    let mut balances: HashMap<UserAddress, IdtAmount> = HashMap::new();

    stack.push((
        root.clone(),
        VisitNode {
            children_visited: false,
            visited_branch: im::HashSet::default(),
        },
    ));

    loop {
        let (user, visit_node) = match stack.pop() {
            None => return Ok(balances.get(root).cloned().unwrap_or_default()),
            Some(x) => x,
        };
        if !visit_node.children_visited {
            let mut visited_branch = visit_node.visited_branch;
            visited_branch.insert(user.clone());
            stack.push((
                user.clone(),
                VisitNode {
                    children_visited: true,
                    // each node has own visited branch because we do not want
                    // other branches to affect balance calculation of the current
                    // branch
                    visited_branch: visited_branch.clone(),
                },
            ));
            for v in tree.children(&user).await? {
                // Skip nodes that have already been visited to avoid cycles in the tree traversal
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
        let user_balance = tree
            .exit_node(&user, &visit_node.visited_branch, &balances)
            .await?;
        balances.insert(user, user_balance);
    }
}
