use async_trait::async_trait;

use crate::{admins::error::Error, identity::UserAddress};
use std::{collections::HashSet, sync::RwLock};

pub mod error;

#[async_trait]
pub trait AdminStorage: Send + Sync {
    async fn is_admin(&self, user: &UserAddress) -> Result<(), Error>;
    async fn is_moderator(&self, user: &UserAddress) -> Result<(), Error>;
    async fn add_admin(&self, caller: &UserAddress, new_admin: UserAddress) -> Result<(), Error>;
    async fn remove_admin(&self, caller: &UserAddress, admin: UserAddress) -> Result<(), Error>;
    async fn add_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error>;
    async fn remove_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error>;
}

// In-memory implementation of AdminStorage
#[derive(Default)]
pub struct InMemoryAdminStorage {
    admins: RwLock<HashSet<UserAddress>>,
    moderators: RwLock<HashSet<UserAddress>>,
}

impl InMemoryAdminStorage {
    pub fn new(admins: HashSet<UserAddress>, moderators: HashSet<UserAddress>) -> Self {
        Self {
            admins: RwLock::new(admins),
            moderators: RwLock::new(moderators),
        }
    }
}

#[async_trait]
impl AdminStorage for InMemoryAdminStorage {
    async fn is_admin(&self, user: &UserAddress) -> Result<(), Error> {
        if self
            .admins
            .read()
            .expect("Poisoned RwLock detected")
            .contains(user)
        {
            return Ok(());
        }
        Err(Error::NoAdminPriviledge)
    }

    async fn is_moderator(&self, user: &UserAddress) -> Result<(), Error> {
        if self
            .moderators
            .read()
            .expect("Poisoned RwLock detected")
            .contains(user)
        {
            return Ok(());
        }
        Err(Error::NoAdminPriviledge)
    }

    async fn add_admin(&self, caller: &UserAddress, new_admin: UserAddress) -> Result<(), Error> {
        let mut admins_lock = self.admins.write().expect("Poisoned RwLock detected");
        // do not use is_admin() since we want to check for admin rights and
        // update admins atomically
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        admins_lock.insert(new_admin);
        Ok(())
    }

    async fn remove_admin(&self, caller: &UserAddress, admin: UserAddress) -> Result<(), Error> {
        let mut admins_lock = self.admins.write().expect("Poisoned RwLock detected");
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        admins_lock.remove(&admin);
        Ok(())
    }

    async fn add_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error> {
        let admins_lock = self.admins.read().expect("Poisoned RwLock detected");
        if !admins_lock.contains(caller) {
            return Err(Error::NoAdminPriviledge);
        }
        self.moderators
            .write()
            .expect("Poisoned RwLock detected")
            .insert(moderator);
        Ok(())
    }

    async fn remove_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error> {
        let admins_lock = self.admins.read().expect("Poisoned RwLock detected");
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

    #[async_std::test]
    async fn test_basic() {
        let storage = InMemoryAdminStorage::default();
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
        assert!(storage.is_admin(&admin).await.is_ok());
        assert!(storage.is_admin(&moderator).await.is_err());
        assert!(storage.is_admin(&regular_user).await.is_err());
    }

    #[async_std::test]
    async fn test_moderator_management() {
        let storage = InMemoryAdminStorage::default();
        let admin = "admin".to_string();
        let moderator = "moderator".to_string();

        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin.clone());

        // Add moderator
        assert!(
            storage
                .add_moderator(&admin, moderator.clone())
                .await
                .is_ok()
        );
        assert!(storage.is_moderator(&moderator).await.is_ok());

        // Non-admin can't add moderator
        let another_user = "another".to_string();
        assert!(
            storage
                .add_moderator(&another_user, "new_mod".to_string())
                .await
                .is_err()
        );

        // Remove moderator
        assert!(
            storage
                .remove_moderator(&admin, moderator.clone())
                .await
                .is_ok()
        );
        assert!(storage.is_moderator(&moderator).await.is_err());
        assert!(
            !storage
                .moderators
                .read()
                .expect("Poisoned RwLock detected")
                .contains(&moderator)
        );
    }

    #[async_std::test]
    async fn test_admin_management() {
        let storage = InMemoryAdminStorage::default();
        let admin1 = "admin1".to_string();
        let admin2 = "admin2".to_string();

        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin1.clone());

        // Admin can add another admin
        assert!(storage.add_admin(&admin1, admin2.clone()).await.is_ok());
        assert!(storage.is_admin(&admin2).await.is_ok());

        // Only admins can add admins
        let non_admin = "non_admin".to_string();
        assert!(
            storage
                .add_admin(&non_admin, "new_admin".to_string())
                .await
                .is_err()
        );

        // Admin can remove another admin
        assert!(storage.remove_admin(&admin1, admin2.clone()).await.is_ok());
        assert!(storage.is_admin(&admin2).await.is_err());
    }

    #[async_std::test]
    async fn test_edge_cases() {
        let storage = InMemoryAdminStorage::default();
        let admin = "admin".to_string();

        storage
            .admins
            .write()
            .expect("Poisoned RwLock detected")
            .insert(admin.clone());

        // Rremoving non-existent moderator should still return Ok
        let non_existent = "non_existent".to_string();
        assert!(
            storage
                .remove_moderator(&admin, non_existent.clone())
                .await
                .is_ok()
        );

        // adding existing moderator should still work
        let moderator = "moderator".to_string();
        assert!(
            storage
                .add_moderator(&admin, moderator.clone())
                .await
                .is_ok()
        );
        assert!(
            storage
                .add_moderator(&admin, moderator.clone())
                .await
                .is_ok()
        );

        // Moderator can't add or remove other moderators
        assert!(
            storage
                .add_moderator(&moderator, "new_mod".to_string())
                .await
                .is_err()
        );
        assert!(
            storage
                .remove_moderator(&moderator, "some_mod".to_string())
                .await
                .is_err()
        );
    }
}
