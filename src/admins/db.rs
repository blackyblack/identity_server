use std::collections::HashSet;

use async_trait::async_trait;
use sqlx::AnyPool;
use sqlx::any::AnyPoolOptions;

use crate::admins::{AdminStorage, error::Error};
use crate::identity::UserAddress;

pub struct DatabaseAdminStorage {
    pool: AnyPool,
}

impl DatabaseAdminStorage {
    pub async fn new(
        url: &str,
        admins: HashSet<UserAddress>,
        moderators: HashSet<UserAddress>,
    ) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS admins (user TEXT PRIMARY KEY)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS moderators (user TEXT PRIMARY KEY)")
            .execute(&pool)
            .await?;
        for admin in admins {
            sqlx::query("INSERT OR IGNORE INTO admins (user) VALUES (?)")
                .bind(admin)
                .execute(&pool)
                .await?;
        }
        for moderator in moderators {
            sqlx::query("INSERT OR IGNORE INTO moderators (user) VALUES (?)")
                .bind(moderator)
                .execute(&pool)
                .await?;
        }
        Ok(Self { pool })
    }
}

#[async_trait]
impl AdminStorage for DatabaseAdminStorage {
    async fn check_admin(&self, user: &UserAddress) -> Result<(), Error> {
        let row = sqlx::query("SELECT user FROM admins WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        if row.is_some() {
            return Ok(());
        }
        Err(Error::NoAdminPrivilege)
    }

    async fn check_moderator(&self, user: &UserAddress) -> Result<(), Error> {
        let row = sqlx::query("SELECT user FROM moderators WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        if row.is_some() {
            return Ok(());
        }
        Err(Error::NoModeratorPrivilege)
    }

    async fn add_admin(&self, caller: &UserAddress, new_admin: UserAddress) -> Result<(), Error> {
        self.check_admin(caller).await?;
        sqlx::query("INSERT OR IGNORE INTO admins (user) VALUES (?)")
            .bind(new_admin)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_admin(&self, caller: &UserAddress, admin: UserAddress) -> Result<(), Error> {
        self.check_admin(caller).await?;
        sqlx::query("DELETE FROM admins WHERE user = ?")
            .bind(admin)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error> {
        self.check_admin(caller).await?;
        sqlx::query("INSERT OR IGNORE INTO moderators (user) VALUES (?)")
            .bind(moderator)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_moderator(
        &self,
        caller: &UserAddress,
        moderator: UserAddress,
    ) -> Result<(), Error> {
        self.check_admin(caller).await?;
        sqlx::query("DELETE FROM moderators WHERE user = ?")
            .bind(moderator)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let admin = "admin".to_string();
        let moderator = "mod".to_string();
        let user = "user".to_string();
        let admins = HashSet::from([admin.clone()]);
        let moderators = HashSet::from([moderator.clone()]);
        let storage = DatabaseAdminStorage::new("sqlite::memory:", admins, moderators)
            .await
            .unwrap();

        assert!(storage.check_admin(&admin).await.is_ok());
        assert!(storage.check_admin(&user).await.is_err());
        assert!(storage.check_admin(&moderator).await.is_err());
        assert!(storage.check_moderator(&moderator).await.is_ok());
        assert!(storage.check_moderator(&user).await.is_err());
        assert!(storage.check_moderator(&admin).await.is_err());

        let admin2 = "admin2".to_string();
        storage.add_admin(&admin, admin2.clone()).await.unwrap();
        assert!(storage.check_admin(&admin2).await.is_ok());
        storage.remove_admin(&admin, admin2.clone()).await.unwrap();
        assert!(storage.check_admin(&admin2).await.is_err());

        let mod2 = "mod2".to_string();
        storage.add_moderator(&admin, mod2.clone()).await.unwrap();
        assert!(storage.check_moderator(&mod2).await.is_ok());
        storage
            .remove_moderator(&admin, mod2.clone())
            .await
            .unwrap();
        assert!(storage.check_moderator(&mod2).await.is_err());
    }
}
