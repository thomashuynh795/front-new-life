use sea_orm::sea_query::{Alias, ForeignKey, ForeignKeyAction, Index, Table};
use sea_orm::{ConnectionTrait, Database, DbConn, Schema};

use std::env;
use std::path::PathBuf;

use database_model::{audit_event, item, scan_event, scan_token, tag, token_batch};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| anyhow::anyhow!("workspace root should exist"))?
        .to_path_buf();
    let env_file = workspace_root.join(".env");
    dotenvy::from_path(&env_file).map_err(|error| {
        anyhow::anyhow!("Failed to load .env from {}: {}", env_file.display(), error)
    })?;

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required in the root .env"))?;

    let db: DbConn = Database::connect(&database_url).await?;

    drop_table_if_exists(&db, "audit_events").await?;
    drop_table_if_exists(&db, "scan_events").await?;
    drop_table_if_exists(&db, "scan_tokens").await?;
    drop_table_if_exists(&db, "token_batches").await?;
    drop_table_if_exists(&db, "items").await?;
    drop_table_if_exists(&db, "tags").await?;

    let schema = Schema::new(db.get_database_backend());

    create_table(&db, schema.create_table_from_entity(tag::Entity)).await?;
    create_table(&db, schema.create_table_from_entity(item::Entity)).await?;
    create_table(&db, schema.create_table_from_entity(token_batch::Entity)).await?;
    create_table(&db, schema.create_table_from_entity(scan_token::Entity)).await?;
    create_table(&db, schema.create_table_from_entity(scan_event::Entity)).await?;
    create_table(&db, schema.create_table_from_entity(audit_event::Entity)).await?;

    add_fk_items_tag(&db).await?;
    add_fk_token_batches_tag(&db).await?;
    add_fk_scan_tokens_batch(&db).await?;
    add_fk_scan_tokens_tag(&db).await?;
    add_fk_scan_events_tag(&db).await?;
    add_fk_scan_events_token(&db).await?;
    add_fk_audit_events_tag(&db).await?;
    add_indexes(&db).await?;

    Ok(())
}

async fn drop_table_if_exists(db: &DbConn, table_name: &str) -> anyhow::Result<()> {
    let stmt = Table::drop()
        .table(Alias::new(table_name))
        .if_exists()
        .cascade()
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn create_table(
    db: &DbConn,
    stmt: sea_orm::sea_query::TableCreateStatement,
) -> anyhow::Result<()> {
    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_items_tag(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_items_tag")
        .from(Alias::new("items"), Alias::new("tag_id"))
        .to(Alias::new("tags"), Alias::new("id"))
        .on_delete(ForeignKeyAction::Restrict)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_scan_events_tag(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_scan_events_tag")
        .from(Alias::new("scan_events"), Alias::new("tag_id"))
        .to(Alias::new("tags"), Alias::new("id"))
        .on_delete(ForeignKeyAction::Cascade)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_token_batches_tag(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_token_batches_tag")
        .from(Alias::new("token_batches"), Alias::new("tag_id"))
        .to(Alias::new("tags"), Alias::new("id"))
        .on_delete(ForeignKeyAction::SetNull)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_scan_tokens_batch(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_scan_tokens_batch")
        .from(Alias::new("scan_tokens"), Alias::new("batch_id"))
        .to(Alias::new("token_batches"), Alias::new("id"))
        .on_delete(ForeignKeyAction::SetNull)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_scan_tokens_tag(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_scan_tokens_tag")
        .from(Alias::new("scan_tokens"), Alias::new("tag_id"))
        .to(Alias::new("tags"), Alias::new("id"))
        .on_delete(ForeignKeyAction::SetNull)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_scan_events_token(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_scan_events_token")
        .from(Alias::new("scan_events"), Alias::new("token_id"))
        .to(Alias::new("scan_tokens"), Alias::new("token_id"))
        .on_delete(ForeignKeyAction::SetNull)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_fk_audit_events_tag(db: &DbConn) -> anyhow::Result<()> {
    let stmt = ForeignKey::create()
        .name("fk_audit_events_tag")
        .from(Alias::new("audit_events"), Alias::new("tag_id"))
        .to(Alias::new("tags"), Alias::new("id"))
        .on_delete(ForeignKeyAction::SetNull)
        .to_owned();

    db.execute(db.get_database_backend().build(&stmt)).await?;
    Ok(())
}

async fn add_indexes(db: &DbConn) -> anyhow::Result<()> {
    let index_statements = [
        Index::create()
            .name("idx_tags_tag_uid")
            .table(Alias::new("tags"))
            .col(Alias::new("tag_uid"))
            .unique()
            .to_owned(),
        Index::create()
            .name("idx_scan_tokens_product_status")
            .table(Alias::new("scan_tokens"))
            .col(Alias::new("product_public_id"))
            .col(Alias::new("status"))
            .to_owned(),
        Index::create()
            .name("idx_scan_tokens_token_hash")
            .table(Alias::new("scan_tokens"))
            .col(Alias::new("token_hash"))
            .unique()
            .to_owned(),
        Index::create()
            .name("idx_scan_tokens_pid_hash")
            .table(Alias::new("scan_tokens"))
            .col(Alias::new("product_public_id"))
            .col(Alias::new("token_hash"))
            .to_owned(),
        Index::create()
            .name("idx_scan_tokens_tag_id")
            .table(Alias::new("scan_tokens"))
            .col(Alias::new("tag_id"))
            .to_owned(),
        Index::create()
            .name("idx_token_batches_tag_status")
            .table(Alias::new("token_batches"))
            .col(Alias::new("tag_id"))
            .col(Alias::new("status"))
            .to_owned(),
        Index::create()
            .name("idx_scan_tokens_expires_at")
            .table(Alias::new("scan_tokens"))
            .col(Alias::new("expires_at"))
            .to_owned(),
        Index::create()
            .name("idx_scan_events_tag_uid")
            .table(Alias::new("scan_events"))
            .col(Alias::new("tag_uid"))
            .to_owned(),
        Index::create()
            .name("idx_scan_events_product_public_id")
            .table(Alias::new("scan_events"))
            .col(Alias::new("product_public_id"))
            .to_owned(),
        Index::create()
            .name("idx_scan_events_token_id")
            .table(Alias::new("scan_events"))
            .col(Alias::new("token_id"))
            .to_owned(),
        Index::create()
            .name("idx_audit_events_tag_id")
            .table(Alias::new("audit_events"))
            .col(Alias::new("tag_id"))
            .to_owned(),
    ];

    for stmt in index_statements {
        db.execute(db.get_database_backend().build(&stmt)).await?;
    }

    Ok(())
}
