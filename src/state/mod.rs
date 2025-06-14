use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type UserAddress = String;
// tuple of voucher or vouchee and vouch timestamp in seconds
pub type VouchEvent = (UserAddress, u64);
pub type ProofId = u128;
pub type IdtAmount = u128;

#[derive(Default)]
struct VouchersState {
    // key - vouchee, vouch object
    // value - (voucher, unix timestamp) map
    pub vouchers: HashMap<UserAddress, HashMap<UserAddress, u64>>,
    // key - voucher, vouch subject
    // value - (vouchee, unix timestamp) map
    pub vouchees: HashMap<UserAddress, HashMap<UserAddress, u64>>,
}

#[derive(Clone)]
pub struct ProofEvent {
    pub moderator: UserAddress,
    pub idt_balance: IdtAmount,
    pub proof_id: ProofId,
    pub timestamp: u64,
}

// key - proven user
#[derive(Default)]
pub struct ProofState(HashMap<UserAddress, ProofEvent>);

// key - punished user
#[derive(Default)]
pub struct PenaltyState(HashMap<UserAddress, ProofEvent>);

#[derive(Default, Clone)]
pub struct State {
    vouchers: Arc<RwLock<VouchersState>>,
    proofs: Arc<RwLock<ProofState>>,
    penalties: Arc<RwLock<PenaltyState>>,
}

impl VouchersState {
    pub fn vouch(&mut self, from: UserAddress, to: UserAddress, timestamp: u64) {
        self.vouchers
            .entry(to.clone())
            .and_modify(|v| {
                v.insert(from.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(from.clone(), timestamp);
                m
            });
        self.vouchees
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

    pub fn vouchers(&self, user: &UserAddress) -> HashMap<UserAddress, u64> {
        self.vouchers.get(user).cloned().unwrap_or_default()
    }

    pub fn vouchees(&self, user: &UserAddress) -> HashMap<UserAddress, u64> {
        self.vouchees.get(user).cloned().unwrap_or_default()
    }
}

impl ProofState {
    pub fn prove(&mut self, user: UserAddress, event: ProofEvent) {
        self.0.insert(user, event);
    }

    pub fn proof(&self, user: &UserAddress) -> Option<ProofEvent> {
        self.0.get(user).cloned()
    }
}

impl PenaltyState {
    pub fn punish(&mut self, user: UserAddress, event: ProofEvent) {
        self.0.insert(user, event);
    }

    pub fn penalty(&self, user: &UserAddress) -> Option<ProofEvent> {
        self.0.get(user).cloned()
    }
}

impl State {
    pub fn vouch(&mut self, from: UserAddress, to: UserAddress, timestamp: u64) {
        self.vouchers
            .write()
            .expect("Poisoned RwLock detected")
            .vouch(from, to, timestamp);
    }

    pub fn voucher_timestamp(&self, user: &UserAddress, voucher: &UserAddress) -> Option<u64> {
        self.vouchers
            .read()
            .expect("Poisoned RwLock detected")
            .vouchers(user)
            .get(voucher)
            .copied()
    }

    pub fn vouchers_with_time(&self, user: &UserAddress) -> Vec<VouchEvent> {
        self.vouchers
            .read()
            .expect("Poisoned RwLock detected")
            .vouchers(user)
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    pub fn vouchers(&self, user: &UserAddress) -> Vec<UserAddress> {
        self.vouchers_with_time(user)
            .iter()
            .map(|(k, _v)| k.clone())
            .collect()
    }

    pub fn vouchee_timestamp(&self, user: &UserAddress, vouchee: &UserAddress) -> Option<u64> {
        self.vouchers
            .read()
            .expect("Poisoned RwLock detected")
            .vouchees(user)
            .get(vouchee)
            .copied()
    }

    pub fn vouchees_with_time(&self, user: &UserAddress) -> Vec<VouchEvent> {
        self.vouchers
            .read()
            .expect("Poisoned RwLock detected")
            .vouchees(user)
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    pub fn vouchees(&self, user: &UserAddress) -> Vec<UserAddress> {
        self.vouchees_with_time(user)
            .iter()
            .map(|(k, _v)| k.clone())
            .collect()
    }

    pub fn prove(
        &mut self,
        user: UserAddress,
        moderator: UserAddress,
        balance: IdtAmount,
        proof_id: ProofId,
        timestamp: u64,
    ) {
        let proof_event = ProofEvent {
            moderator,
            idt_balance: balance,
            proof_id,
            timestamp,
        };
        self.proofs
            .write()
            .expect("Poisoned RwLock detected")
            .prove(user, proof_event);
    }

    pub fn proof_event(&self, user: &UserAddress) -> Option<ProofEvent> {
        self.proofs
            .read()
            .expect("Poisoned RwLock detected")
            .proof(user)
    }

    pub fn punish(
        &mut self,
        user: UserAddress,
        moderator: UserAddress,
        balance: IdtAmount,
        proof_id: ProofId,
        timestamp: u64,
    ) {
        let proof_event = ProofEvent {
            moderator,
            idt_balance: balance,
            proof_id,
            timestamp,
        };
        self.penalties
            .write()
            .expect("Poisoned RwLock detected")
            .punish(user, proof_event);
    }

    pub fn penalty_event(&self, user: &UserAddress) -> Option<ProofEvent> {
        self.penalties
            .read()
            .expect("Poisoned RwLock detected")
            .penalty(user)
    }
}
