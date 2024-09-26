// @generated automatically by Diesel CLI.

diesel::table! {
    multiaddress (id) {
        id -> Int4,
        address -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    service_state (id) {
        id -> Int4,
        last_processed_block -> Int8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    multiaddress,
    service_state,
);
