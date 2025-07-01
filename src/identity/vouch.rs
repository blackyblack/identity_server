use std::collections::HashMap;

use crate::identity::{IdentityService, UserAddress, next_timestamp};

impl IdentityService {
    pub fn vouch_with_timestamp(&self, from: UserAddress, to: UserAddress, timestamp: u64) {
        self.vouches.vouch(from, to, timestamp);
    }

    pub fn vouchers_with_time(&self, user: &UserAddress) -> HashMap<UserAddress, u64> {
        self.vouches.vouchers_with_time(user)
    }

    pub fn vouchees_with_time(&self, user: &UserAddress) -> HashMap<UserAddress, u64> {
        self.vouches.vouchees_with_time(user)
    }
}

pub fn vouch(service: &IdentityService, from: UserAddress, to: UserAddress) {
    service.vouch_with_timestamp(from, to, next_timestamp());
}

pub fn vouchers(service: &IdentityService, user: &UserAddress) -> Vec<UserAddress> {
    service.vouchers_with_time(user).into_keys().collect()
}

pub fn vouchees(service: &IdentityService, user: &UserAddress) -> Vec<UserAddress> {
    service.vouchees_with_time(user).into_keys().collect()
}

pub fn voucher_timestamp(
    service: &IdentityService,
    user: &UserAddress,
    voucher: &UserAddress,
) -> Option<u64> {
    service.vouchers_with_time(user).get(voucher).copied()
}

pub fn vouchee_timestamp(
    service: &IdentityService,
    user: &UserAddress,
    vouchee: &UserAddress,
) -> Option<u64> {
    service.vouchees_with_time(user).get(vouchee).copied()
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
