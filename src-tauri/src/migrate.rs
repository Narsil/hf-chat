use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    //  Setting `DATABASE_URL` environment variable
    let key = "DATABASE_URL";
    let value = "sqlite:./db.sqlite";
    std::env::set_var(key, value);
    cli::run_cli(app::migrations::Migrator).await;
}

