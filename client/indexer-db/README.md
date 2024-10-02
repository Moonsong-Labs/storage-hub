# Storage Hub Indexer Database

This crate provides a Diesel ORM for the Storage Hub Indexer database.

## Setup (macOS)

1. Install `libpq`:

```sh
brew install libpq
```

2. Update the environment paths to include the `libpq` location:

```sh
echo 'export PATH="/opt/homebrew/opt/libpq/bin:$PATH"' >> ~/.zshrc
echo 'export LDFLAGS="-L/opt/homebrew/opt/libpq/lib"' >> ~/.zshrc
echo 'export CPPFLAGS="-I/opt/homebrew/opt/libpq/include"' >> ~/.zshrc
echo 'export PKG_CONFIG_PATH="/opt/homebrew/opt/libpq/lib/pkgconfig"' >> ~/.zshrc
source ~/.zshrc
```

3. Install diesel CLI:

```sh
cargo install diesel_cli --no-default-features --features postgres
```

4. Start Postgres. For development a docker compose file is provided.

```sh
docker compose -f ../../docker/local-dev-postgres.yml up
```

5. Export the database URL to the environment:

```sh
export DATABASE_URL="postgresql://superuser:superpassword@localhost:5432/shc-indexer"
```

6. Setup the database:

```sh
diesel setup
```
This will create the database and apply the migrations.

## Usage

To use the Diesel ORM in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
shc-indexer-db = { path = "../indexer-db" }
```

Then, you can use the Diesel ORM to interact with the database.

## Migrations

To create a new migration, run the following command:

```sh
diesel migration generate <migration_name>
```

This will create a new migration directory containing an `up.sql` and `down.sql` files which will
contain the SQL statements to apply and undo the migration.

To apply the migration, run the following command:

```sh
diesel migration run
```

If the schema is not generated automatically when applying migrations, run the following command to 
manually generate it:

```sh
diesel print-schema > src/schema.rs
```
