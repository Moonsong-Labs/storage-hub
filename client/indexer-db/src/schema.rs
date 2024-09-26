// @generated automatically by Diesel CLI.

diesel::table! {
    bsp (id) {
        id -> Int4,
        account -> Varchar,
        capacity -> Numeric,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    bsp_multiaddress (bsp_id, multiaddress_id) {
        bsp_id -> Int4,
        multiaddress_id -> Int4,
    }
}

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

diesel::joinable!(bsp_multiaddress -> bsp (bsp_id));
diesel::joinable!(bsp_multiaddress -> multiaddress (multiaddress_id));

diesel::allow_tables_to_appear_in_same_query!(
    bsp,
    bsp_multiaddress,
    multiaddress,
    service_state,
);
