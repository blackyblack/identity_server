use crate::identity::{
    IdentityService, IdtAmount, SystemPenalty, UserAddress, next_timestamp,
    vouch::voucher_timestamp,
};

fn flat_one_idt_decay(event_timestamp: u64) -> IdtAmount {
    let now = next_timestamp();
    // future timestamp, should not happen
    if now < event_timestamp {
        return 0;
    }
    // decay is 1 IDT per day
    ((now - event_timestamp) as u128) / 60 / 60 / 24
}

pub fn proof_decay(service: &IdentityService, user: &UserAddress) -> IdtAmount {
    let (timestamp, _balance) = match service.proof(user) {
        None => return 0,
        Some(e) => (e.timestamp, e.amount),
    };
    flat_one_idt_decay(timestamp)
}

pub fn moderator_penalty_decay(service: &IdentityService, user: &UserAddress) -> IdtAmount {
    let (timestamp, _penalty) = match service.moderator_penalty(user) {
        None => return 0,
        Some(e) => (e.timestamp, e.amount),
    };
    flat_one_idt_decay(timestamp)
}

// vouchers decay twice,
// first, from proof_decay, vouchee gets 0.1 of the voucher's proof_decay this way
// second, from vouch_decay, vouchee gets 1 IDT per day decay from each voucher balance
pub fn vouch_decay(
    service: &IdentityService,
    user: &UserAddress,
    voucher: &UserAddress,
) -> IdtAmount {
    let timestamp = match voucher_timestamp(service, user, voucher) {
        None => return 0,
        Some(e) => e,
    };
    flat_one_idt_decay(timestamp)
}

pub fn system_penalty_decay(event: &SystemPenalty) -> IdtAmount {
    flat_one_idt_decay(event.timestamp)
}

// subtract decay from balance, ensuring it does not go below zero
pub fn balance_after_decay(balance: IdtAmount, decay: IdtAmount) -> IdtAmount {
    balance.saturating_sub(decay)
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::{MODERATOR, PROOF_ID, USER_A};

    use super::*;

    #[test]
    fn test_basic_proof_decay() {
        let service = IdentityService::default();
        let ts = next_timestamp();
        assert_eq!(proof_decay(&service, &USER_A.to_string()), 0);
        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts,
        );
        assert_eq!(proof_decay(&service, &USER_A.to_string()), 0);
        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts - 86400,
        );
        assert_eq!(proof_decay(&service, &USER_A.to_string()), 1);
        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts - 100000,
        );
        assert_eq!(proof_decay(&service, &USER_A.to_string()), 1);
        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts - 86400 * 2,
        );
        assert_eq!(proof_decay(&service, &USER_A.to_string()), 2);
    }

    #[test]
    fn test_basic_penalty_decay() {
        let service = IdentityService::default();
        let ts = next_timestamp();
        assert_eq!(moderator_penalty_decay(&service, &USER_A.to_string()), 0);
        service.punish_with_timestamp(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts);
        assert_eq!(moderator_penalty_decay(&service, &USER_A.to_string()), 0);
        service.punish_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts - 86400,
        );
        assert_eq!(moderator_penalty_decay(&service, &USER_A.to_string()), 1);
        service.punish_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts - 100000,
        );
        assert_eq!(moderator_penalty_decay(&service, &USER_A.to_string()), 1);
        service.punish_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
            ts - 86400 * 2,
        );
        assert_eq!(moderator_penalty_decay(&service, &USER_A.to_string()), 2);
    }

    #[test]
    fn test_basic_vouch_decay() {
        let user_b = "userB";
        let service = IdentityService::default();
        let ts = next_timestamp();
        assert_eq!(
            vouch_decay(&service, &USER_A.to_string(), &user_b.to_string()),
            0
        );
        service.vouch_with_timestamp(user_b.to_string(), USER_A.to_string(), ts);
        assert_eq!(
            vouch_decay(&service, &USER_A.to_string(), &user_b.to_string()),
            0
        );
        service.vouch_with_timestamp(user_b.to_string(), USER_A.to_string(), ts - 86400);
        assert_eq!(
            vouch_decay(&service, &USER_A.to_string(), &user_b.to_string()),
            1
        );
        service.vouch_with_timestamp(user_b.to_string(), USER_A.to_string(), ts - 100000);
        assert_eq!(
            vouch_decay(&service, &USER_A.to_string(), &user_b.to_string()),
            1
        );
        service.vouch_with_timestamp(user_b.to_string(), USER_A.to_string(), ts - 86400 * 2);
        assert_eq!(
            vouch_decay(&service, &USER_A.to_string(), &user_b.to_string()),
            2
        );
    }

    #[test]
    fn test_system_penalty_decay() {
        let ts = next_timestamp();
        assert_eq!(
            system_penalty_decay(&SystemPenalty {
                amount: 10,
                timestamp: ts
            }),
            0
        );
        assert_eq!(
            system_penalty_decay(&SystemPenalty {
                amount: 10,
                timestamp: ts - 86400
            }),
            1
        );
        assert_eq!(
            system_penalty_decay(&SystemPenalty {
                amount: 10,
                timestamp: ts - 86400 * 2
            }),
            2
        );
    }
}
