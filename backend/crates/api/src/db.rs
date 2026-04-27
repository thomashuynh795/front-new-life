use sea_orm::{Database, DatabaseConnection, DbErr};

/// Opens a `SeaORM` database connection.
///
/// # Errors
///
/// Returns the database driver error when the connection cannot be established.
pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
}
