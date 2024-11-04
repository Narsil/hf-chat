use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Message::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Message::Content).string().not_null())
                    .col(ColumnDef::new(Message::UserId).uuid())
                    .foreign_key(
                        sea_query::ForeignKey::create()
                            .name("fk-message-user_id")
                            .from(Message::Table, Message::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Message::ConversationId).uuid().not_null())
                    .foreign_key(
                        sea_query::ForeignKey::create()
                            .name("fk-message-conversation_id")
                            .from(Message::Table, Message::ConversationId)
                            .to(Conversation::Table, Conversation::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Message::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Message::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Message::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Message {
    Table,
    Id,
    Content,
    UserId,
    ConversationId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Conversation {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
