pub use sea_orm_migration::prelude::*;

mod m20230914_053359_create_model;
mod m20230918_064025_create_conversation;
mod m20230918_070745_initial_models;
mod m20230918_082713_create_messages;
mod m20230920_064642_create_settings;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230914_053359_create_model::Migration),
            Box::new(m20230918_064025_create_conversation::Migration),
            Box::new(m20230918_070745_initial_models::Migration),
            Box::new(m20230918_082713_create_messages::Migration),
            Box::new(m20230920_064642_create_settings::Migration),
        ]
    }
}
