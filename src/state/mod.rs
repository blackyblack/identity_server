use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
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

#[derive(Default)]
struct ProofsState {
    // key - proven user
    // value - ProofEvent
    pub proofs: HashMap<UserAddress, ProofEvent>,
}

#[derive(Default, Clone)]
pub struct State {
    vouchers: Arc<RwLock<VouchersState>>,
    proofs: Arc<RwLock<ProofsState>>,
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

impl ProofsState {
    pub fn prove(&mut self, user: UserAddress, event: ProofEvent) {
        self.proofs.insert(user, event);
    }

    pub fn proof(&self, user: &UserAddress) -> Option<ProofEvent> {
        self.proofs.get(user).cloned()
    }
}

impl State {
    pub fn vouch(&mut self, from: UserAddress, to: UserAddress) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Should be after the UNIX_EPOCH timestamp")
            .as_secs();
        self.vouchers
            .write()
            .expect("Poisoned RwLock detected")
            .vouch(from, to, ts);
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
    ) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Should be after the UNIX_EPOCH timestamp")
            .as_secs();
        let proof_event = ProofEvent {
            moderator,
            idt_balance: balance,
            proof_id,
            timestamp: ts,
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
}
