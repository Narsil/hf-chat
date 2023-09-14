pub use sea_orm_migration::prelude::*;

mod m20230914_053359_create_model;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20230914_053359_create_model::Migration)]
    }
}
