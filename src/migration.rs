use sqlx::{AnyPool, Error, migrate::Migrator};

// embed migrations from the crate root `migrations` directory
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(pool: &AnyPool) -> Result<(), Error> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| Error::Migrate(Box::new(e)))
}
