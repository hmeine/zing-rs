pub use sea_orm_migration::prelude::*;

mod m20230924_000001_create_user_table;
mod m20231004_134936_m20231004_000002_create_table_table;
mod m20231005_153749_m20231005_000001_add_game_results_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230924_000001_create_user_table::Migration),
            Box::new(m20231004_134936_m20231004_000002_create_table_table::Migration),
            Box::new(m20231005_153749_m20231005_000001_add_game_results_table::Migration),
        ]
    }
}
