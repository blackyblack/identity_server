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
        admins: impl IntoIterator<Item = UserAddress>,
        moderators: impl IntoIterator<Item = UserAddress>,
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
    async fn is_admin(&self, user: &UserAddress) -> Result<(), Error> {
        let row = sqlx::query("SELECT user FROM admins WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        if row.is_some() {
            Ok(())
        } else {
            Err(Error::NoAdminPrivilege)
        }
    }

    async fn is_moderator(&self, user: &UserAddress) -> Result<(), Error> {
        let row = sqlx::query("SELECT user FROM moderators WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        if row.is_some() {
            Ok(())
        } else {
            Err(Error::NoAdminPrivilege)
        }
    }

    async fn add_admin(&self, caller: &UserAddress, new_admin: UserAddress) -> Result<(), Error> {
        self.is_admin(caller).await?;
        sqlx::query("INSERT OR IGNORE INTO admins (user) VALUES (?)")
            .bind(new_admin)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_admin(&self, caller: &UserAddress, admin: UserAddress) -> Result<(), Error> {
        self.is_admin(caller).await?;
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
        self.is_admin(caller).await?;
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
        self.is_admin(caller).await?;
        sqlx::query("DELETE FROM moderators WHERE user = ?")
            .bind(moderator)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
