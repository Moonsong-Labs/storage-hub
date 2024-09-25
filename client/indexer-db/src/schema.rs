// @generated automatically by Diesel CLI.

diesel::table! {
    service_state (id) {
        id -> Int4,
        last_processed_block -> Int8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
