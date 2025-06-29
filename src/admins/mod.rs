use crate::{admins::error::Error, identity::UserAddress};
use std::{collections::HashSet, sync::RwLock};

pub mod error;

#[derive(Default)]
pub struct AdminStorage {
    admins: RwLock<HashSet<UserAddress>>,
    moderators: RwLock<HashSet<UserAddress>>,
}

impl AdminStorage {
    pub fn new(admins: HashSet<UserAddress>, moderators: HashSet<UserAddress>) -> Self {
        Self {
            admins: RwLock::new(admins),
            moderators: RwLock::new(moderators),
        }
    }

    pub fn is_admin(&self, user: &UserAddress) -> bool {
        self.admins
            .read()
            .expect("Poisoned RwLock detected")
            .contains(user)
    }

    pub fn is_moderator(&self, user: &UserAddress) -> bool {
        self.moderators
            .read()
            .expect("Poisoned RwLock detected")
            .contains(user)
    }

    pub fn add_admin(&self, caller: &UserAddress, new_admin: UserAddress) -> Result<(), Error> {
        let mut admins_lock = self.admins.write().expect("Poisoned RwLock detected");
        // do not use is_admin() since we want to check for admin rights and
        // update admins atomically
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        admins_lock.insert(new_admin);
        Ok(())
    }

    pub fn remove_admin(&self, caller: &UserAddress, admin: UserAddress) -> Result<(), Error> {
        let mut admins_lock = self.admins.write().expect("Poisoned RwLock detected");
        // do not use is_admin() since we want to check for admin rights and
        // update admins atomically
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        admins_lock.remove(&admin);
        Ok(())
    }

    pub fn add_moderator(&self, caller: &UserAddress, moderator: UserAddress) -> Result<(), Error> {
        let admins_lock = self.admins.read().expect("Poisoned RwLock detected");
        // do not use is_admin() since we want to check for admin rights and
        // update moderators atomically
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        self.moderators
            .write()
            .expect("Poisoned RwLock detected")
            .insert(moderator);
        Ok(())
    }

    pub fn remove_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error> {
        let admins_lock = self.admins.read().expect("Poisoned RwLock detected");
        // do not use is_admin() since we want to check for admin rights and
        // update moderators atomically
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        self.moderators
            .write()
            .expect("Poisoned RwLock detected")
            .remove(&moderator);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let storage = AdminStorage::default();
        let admin = "admin".to_string();
        let moderator = "moderator".to_string();
        let regular_user = "user".to_string();

        // Add initial admin
        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin.clone());

        // Check permissions
        assert!(storage.is_admin(&admin));
        assert!(!storage.is_admin(&moderator));
        assert!(!storage.is_admin(&regular_user));
    }

    #[test]
    fn test_moderator_management() {
        let storage = AdminStorage::default();
        let admin = "admin".to_string();
        let moderator = "moderator".to_string();

        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin.clone());

        // Add moderator
        assert!(storage.add_moderator(&admin, moderator.clone()).is_ok());
        assert!(storage.is_moderator(&moderator));

        // Non-admin can't add moderator
        let another_user = "another".to_string();
        assert!(
            storage
                .add_moderator(&another_user, "new_mod".to_string())
                .is_err()
        );

        // Remove moderator
        assert!(storage.remove_moderator(&admin, moderator.clone()).is_ok());
        assert!(!storage.is_moderator(&moderator));
        assert!(
            !storage
                .moderators
                .read()
                .expect("Poisoned RwLock detected")
                .contains(&moderator)
        );
    }

    #[test]
    fn test_admin_management() {
        let storage = AdminStorage::default();
        let admin1 = "admin1".to_string();
        let admin2 = "admin2".to_string();

        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin1.clone());

        // Admin can add another admin
        assert!(storage.add_admin(&admin1, admin2.clone()).is_ok());
        assert!(storage.is_admin(&admin2));

        // Only admins can add admins
        let non_admin = "non_admin".to_string();
        assert!(
            storage
                .add_admin(&non_admin, "new_admin".to_string())
                .is_err()
        );

        // Admin can remove another admin
        assert!(storage.remove_admin(&admin1, admin2.clone()).is_ok());
        assert!(!storage.is_admin(&admin2));
    }

    #[test]
    fn test_edge_cases() {
        let storage = AdminStorage::default();
        let admin = "admin".to_string();

        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin.clone());

        // Removing non-existent moderator should still return Ok
        let non_existent = "non_existent".to_string();
        assert!(
            storage
                .remove_moderator(&admin, non_existent.clone())
                .is_ok()
        );

        // Adding existing moderator should still work
        let moderator = "moderator".to_string();
        assert!(storage.add_moderator(&admin, moderator.clone()).is_ok());
        assert!(storage.add_moderator(&admin, moderator.clone()).is_ok());

        // Moderator can't add or remove other moderators
        assert!(
            storage
                .add_moderator(&moderator, "new_mod".to_string())
                .is_err()
        );
        assert!(
            storage
                .remove_moderator(&moderator, "some_mod".to_string())
                .is_err()
        );
    }
}
