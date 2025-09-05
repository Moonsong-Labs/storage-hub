use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

// Embed the migrations from the indexer-db crate
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../client/indexer-db/migrations");

pub mod snapshot_move_bucket {
    use hex_literal::hex;

    use crate::constants::rpc::DUMMY_MSP_ID;

    /// This is a snapshot of the move-bucket-test's resulting db
    /// Contains:
    /// * 3 BSPs
    /// * 3 files duplicated in 2 BSPs each
    /// * 1 bucket containing 3 files
    /// * 2 MSPs
    ///
    /// The snapshot contains the required information to re-create it
    /// This snapshot has been taken at commit b78b5648a38849cee1a462a1b938d63d27e68547
    pub const SNAPSHOT_SQL: &str = include_str!("./indexer-db-snapshot.sql");

    /// The ID value of MSP #1
    pub const MSP_ONE_ID: i64 = 1;

    /// The ACCOUNT value MSP #1
    pub const MSP_ONE_ACCOUNT: &str = "5E1rPv1M2mheg6pM57QqU7TZ6eCwbVpiYfyYkrugpBdEzDiU";
    /// The ONCHAIN ID value of MSP #1
    pub const MSP_ONE_ONCHAIN_ID: [u8; 32] = DUMMY_MSP_ID;

    /// The ID value of MSP #2
    pub const MSP_TWO_ID: i64 = 2;

    /// The ACCOUNT value of MSP #2
    pub const MSP_TWO_ACCOUNT: &str = "5CMDKyadzWu6MUwCzBB93u32Z1PPPsV8A1qAy4ydyVWuRzWR";
    /// The ONCHAIN ID value of MSP #2
    pub const MSP_TWO_ONCHAIN_ID: [u8; 32] =
        hex!("0000000000000000000000000000000000000000000000000000000000000301");
    /// The BUCKET IDs belonging to MSP #2
    pub const MSP_TWO_BUCKETS: &[i64] = &[BUCKET_ID];

    /// The ID value of Bucket #1
    pub const BUCKET_ID: i64 = 1;

    /// The ONCHAIN ID value of Bucket #1
    pub const BUCKET_ONCHAIN_ID: [u8; 32] =
        hex!("8E0EC449E3A21DAB7CD1276895DB940843246E336E24B9A4A1A10C8D59511CDE");
    /// The NAME value of Bucket #1
    pub const BUCKET_NAME: &str = "nothingmuch-3";
    /// The ACCOUNT value of Bucket #1
    pub const BUCKET_ACCOUNT: &str = "5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o";
    /// The PRIVATE value of Bucket #1
    pub const BUCKET_PRIVATE: bool = false;

    /// The number of files belonging to Bucket #1
    pub const BUCKET_FILES: usize = 3;

    /// The total number of BSPs
    pub const BSP_NUM: usize = 3;

    /// The ONCHAIN ID value of BSP #1
    pub const BSP_ONE_ONCHAIN_ID: [u8; 32] =
        hex!("2B83B972E63F52ABC0D4146C4AEE1F1EC8AA8E274D2AD1B626529446DA93736C");

    /// The ACCOUNT value of BSP #1
    pub const BSP_ONE_ACCOUNT: &str = "5FHSHEFWHVGDnyiw66DoRUpLyh5RouWkXo9GT1Sjk8qw7MAg";

    /// The FILE KEY of File #1
    pub const FILE_ONE_FILE_KEY: [u8; 32] =
        hex!("E592BCECC540F2363850B6895D82A310A4DD6686F066603CA2ABFB77FC478B0A");

    /// The LOCATION of File #1
    pub const FILE_ONE_LOCATION: &str = "test/whatsup.jpg";
}

/// Setup an indexer-db instance (thru a container) and run the provided raw SQL queries
///
/// # Arguments:
/// * pre_migrations: the raw SQL query to run before migrations
/// * post_migrations: a vector of raw SQL queries to run after migrations
pub async fn setup_test_db(
    pre_migrations: Vec<String>,
    post_migrations: Vec<String>,
) -> (ContainerAsync<Postgres>, String) {
    // Create the Postgres container with custom configuration
    let mut postgres = Postgres::default()
        .with_db_name("storage_hub")
        .with_user("postgres")
        .with_password("postgres");

    for query in pre_migrations {
        postgres = postgres.with_init_sql(query.into_bytes());
    }

    // Start the container
    let container = postgres
        .with_tag("15") // use the same as integration tests
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

    // Run migrations
    let mut conn =
        diesel::PgConnection::establish(&database_url).expect("Failed to connect for migrations");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");

    for (i, query) in post_migrations.into_iter().enumerate() {
        diesel::RunQueryDsl::execute(diesel::sql_query(query), &mut conn)
            .unwrap_or_else(|e| panic!("Failed to execute init SQL #{i}: {e:?}"));
    }

    (container, database_url)
}
