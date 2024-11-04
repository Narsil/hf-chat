use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Conversation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Conversation::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Conversation::Title).string().not_null())
                    .col(ColumnDef::new(Conversation::ModelId).integer())
                    .foreign_key(
                        sea_query::ForeignKey::create()
                            .name("fk-conversation-model_id")
                            .from(Conversation::Table, Conversation::ModelId)
                            .to(Model::Table, Model::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Conversation::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Conversation::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Conversation::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Conversation {
    Table,
    Id,
    Title,
    ModelId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Model {
    Table,
    Id,
}
