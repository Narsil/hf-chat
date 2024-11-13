use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Model::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Model::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Model::UserId).integer().not_null())
                    .foreign_key(
                        sea_query::ForeignKey::create()
                            .name("fk-model-user_id")
                            .from(Model::Table, Model::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Model::Endpoint).string().not_null())
                    .col(ColumnDef::new(Model::Parameters).json().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Model::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Model {
    Table,
    Id,
    UserId,
    Endpoint,
    Parameters,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
