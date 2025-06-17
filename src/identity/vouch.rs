use std::collections::HashMap;

use crate::identity::{IdentityService, UserAddress, VouchEvent, next_timestamp};

impl IdentityService {
    pub fn vouch_with_timestamp(&self, from: UserAddress, to: UserAddress, timestamp: u64) {
        let mut vouches_lock = self.vouches.write().expect("Poisoned RwLock detected");
        vouches_lock
            .vouchers
            .entry(to.clone())
            .and_modify(|v| {
                v.insert(from.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(from.clone(), timestamp);
                m
            });
        vouches_lock
            .vouchees
            .entry(from)
            .and_modify(|v| {
                v.insert(to.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(to, timestamp);
                m
            });
    }

    pub fn voucher_timestamp(&self, user: &UserAddress, voucher: &UserAddress) -> Option<u64> {
        self.vouches
            .read()
            .expect("Poisoned RwLock detected")
            .vouchers
            .get(user)
            .cloned()
            .unwrap_or_default()
            .get(voucher)
            .copied()
    }

    pub fn vouchers_with_time(&self, user: &UserAddress) -> Vec<VouchEvent> {
        self.vouches
            .read()
            .expect("Poisoned RwLock detected")
            .vouchers
            .get(user)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    pub fn vouchee_timestamp(&self, user: &UserAddress, vouchee: &UserAddress) -> Option<u64> {
        self.vouches
            .read()
            .expect("Poisoned RwLock detected")
            .vouchees
            .get(user)
            .cloned()
            .unwrap_or_default()
            .get(vouchee)
            .copied()
    }

    pub fn vouchees_with_time(&self, user: &UserAddress) -> Vec<VouchEvent> {
        self.vouches
            .read()
            .expect("Poisoned RwLock detected")
            .vouchees
            .get(user)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }
}

pub fn vouch(service: &IdentityService, from: UserAddress, to: UserAddress) {
    service.vouch_with_timestamp(from, to, next_timestamp());
}

pub fn vouchers(service: &IdentityService, user: &UserAddress) -> Vec<UserAddress> {
    service
        .vouchers_with_time(user)
        .iter()
        .map(|(k, _v)| k.clone())
        .collect()
}

pub fn vouchees(service: &IdentityService, user: &UserAddress) -> Vec<UserAddress> {
    service
        .vouchees_with_time(user)
        .iter()
        .map(|(k, _v)| k.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::USER_A;

    use super::*;

    #[test]
    fn test_basic() {
        let service = IdentityService::default();
        let user_b = "userB";
        assert!(vouchees(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert!(vouchees(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).len() == 1);
    }

    #[test]
    fn test_vouch_self() {
        let service = IdentityService::default();
        assert!(vouchees(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        // user can vouch for himself
        vouch(&service, USER_A.to_string(), USER_A.to_string());
        assert!(vouchees(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 1);
    }

    #[test]
    fn test_vouch_twice() {
        let service = IdentityService::default();
        let user_b = "userB";
        assert!(vouchees(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert!(vouchees(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).len() == 1);
        // duplicate vouch does not change anything
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert!(vouchees(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).len() == 1);
    }

    #[test]
    fn test_vouch_mutual() {
        let service = IdentityService::default();
        let user_b = "userB";
        assert!(vouchees(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert!(vouchees(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).len() == 1);
        vouch(&service, user_b.to_string(), USER_A.to_string());
        assert!(vouchees(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).len() == 1);
        assert!(vouchees(&service, &user_b.to_string()).len() == 1);
        assert!(vouchers(&service, &user_b.to_string()).len() == 1);
    }
}
