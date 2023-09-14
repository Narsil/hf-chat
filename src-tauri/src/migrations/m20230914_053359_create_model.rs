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
                        ColumnDef::new(Model::InternalId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Model::Id).string().not_null())
                    .col(ColumnDef::new(Model::Name).string().not_null())
                    .col(ColumnDef::new(Model::WebsiteUrl).string().not_null())
                    .col(ColumnDef::new(Model::DatasetName).string().not_null())
                    .col(ColumnDef::new(Model::DisplayName).string().not_null())
                    .col(ColumnDef::new(Model::Description).string().not_null())
                    .col(ColumnDef::new(Model::PromptExamples).string().not_null())
                    .col(ColumnDef::new(Model::Parameters).string().not_null())
                    .col(ColumnDef::new(Model::Preprompt).string().not_null())
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
    InternalId,
    Id,
    Name,
    WebsiteUrl,
    DatasetName,
    DisplayName,
    Description,
    PromptExamples,
    Parameters,
    Preprompt,
}
