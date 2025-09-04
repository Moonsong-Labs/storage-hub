use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;

// Embed the migrations from the indexer-db crate
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../client/indexer-db/migrations");

/// Helper to set up a PostgreSQL container with migrations and optional init SQL
pub async fn setup_test_db(init_sql: Option<&str>) -> (ContainerAsync<Postgres>, String) {
    // Create the Postgres container with custom configuration
    let mut postgres = Postgres::default()
        .with_db_name("storage_hub")
        .with_user("postgres")
        .with_password("postgres");

    // Add init SQL if provided (executed after migrations)
    if let Some(sql) = init_sql {
        // We need to run migrations first, then the init SQL
        // So we'll handle init SQL separately after migrations
    }

    // Start the container
    let container = postgres
        .start()
        .await
        .expect("Failed to start postgres container");

    // Get the connection URL
    let host = container.get_host().await.expect("Failed to get host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");
    let database_url = format!("postgres://postgres:postgres@{}:{}/storage_hub", host, port);

    // Run migrations and init SQL
    {
        use diesel::prelude::*;
        let mut conn = diesel::PgConnection::establish(&database_url)
            .expect("Failed to connect for migrations");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        // Execute init SQL if provided (after migrations)
        if let Some(sql) = init_sql {
            // Parse SQL and execute each statement separately
            // We need to handle comments and multiple statements
            let mut current_statement = String::new();
            for line in sql.lines() {
                let trimmed = line.trim();
                // Skip comment lines
                if trimmed.starts_with("--") || trimmed.is_empty() {
                    continue;
                }
                current_statement.push_str(line);
                current_statement.push('\n');

                // Execute when we hit a semicolon at the end of a line
                if trimmed.ends_with(';') {
                    let statement = current_statement.trim();
                    if !statement.is_empty() {
                        diesel::RunQueryDsl::execute(diesel::sql_query(statement), &mut conn)
                            .expect("Failed to execute init SQL");
                    }
                    current_statement.clear();
                }
            }
            // Execute any remaining statement
            let remaining = current_statement.trim();
            if !remaining.is_empty() {
                diesel::RunQueryDsl::execute(diesel::sql_query(remaining), &mut conn)
                    .expect("Failed to execute init SQL");
            }
        }
    }

    (container, database_url)
}
