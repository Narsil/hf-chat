use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(Settings::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Settings::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Settings::ShareConversationsWithModelAuthors)
                            .boolean()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Settings::EthicsModelAcceptedAt).date_time())
                    .col(ColumnDef::new(Settings::ActiveModel).string())
                    .foreign_key(
                        sea_query::ForeignKey::create()
                            .name("fk-settings-model_id")
                            .from(Settings::Table, Settings::ActiveModel)
                            .to(Model::Table, Model::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::SetNull),
                    )
                    .col(ColumnDef::new(Settings::SearchEnabled).boolean().not_null())
                    .col(ColumnDef::new(Settings::CustomPrompts).json().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(Settings::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Settings {
    Table,
    Id,
    ShareConversationsWithModelAuthors,
    EthicsModelAcceptedAt,
    ActiveModel,
    SearchEnabled,
    CustomPrompts,
}

#[derive(DeriveIden)]
enum Model {
    Table,
    Id,
}
