// @generated automatically by Diesel CLI.

diesel::table! {
    bsp (id) {
        id -> Int4,
        account -> Varchar,
        capacity -> Numeric,
        stake -> Numeric,
        last_tick_proven -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        onchain_bsp_id -> Varchar,
    }
}

diesel::table! {
    bsp_file (bsp_id, file_id) {
        bsp_id -> Int4,
        file_id -> Int4,
    }
}

diesel::table! {
    bsp_multiaddress (bsp_id, multiaddress_id) {
        bsp_id -> Int4,
        multiaddress_id -> Int4,
    }
}

diesel::table! {
    bucket (id) {
        id -> Int4,
        msp_id -> Nullable<Int4>,
        account -> Varchar,
        onchain_bucket_id -> Varchar,
        name -> Bytea,
        collection_id -> Nullable<Varchar>,
        private -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    file (id) {
        id -> Int4,
        account -> Varchar,
        file_key -> Bytea,
        bucket_id -> Int4,
        location -> Bytea,
        fingerprint -> Bytea,
        size -> Int8,
        step -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    file_peer_id (file_id, peer_id) {
        file_id -> Int4,
        peer_id -> Int4,
    }
}

diesel::table! {
    msp (id) {
        id -> Int4,
        account -> Varchar,
        capacity -> Numeric,
        value_prop -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        onchain_msp_id -> Varchar,
    }
}

diesel::table! {
    msp_multiaddress (msp_id, multiaddress_id) {
        msp_id -> Int4,
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
    paymentstream (id) {
        id -> Int4,
        account -> Varchar,
        provider -> Varchar,
        total_amount_paid -> Numeric,
        last_tick_charged -> Int8,
        charged_at_tick -> Int8,
    }
}

diesel::table! {
    peer_id (id) {
        id -> Int4,
        peer -> Bytea,
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

diesel::joinable!(bsp_file -> file (file_id));
diesel::joinable!(bsp_multiaddress -> bsp (bsp_id));
diesel::joinable!(bsp_multiaddress -> multiaddress (multiaddress_id));
diesel::joinable!(bucket -> msp (msp_id));
diesel::joinable!(file_peer_id -> file (file_id));
diesel::joinable!(file_peer_id -> peer_id (peer_id));
diesel::joinable!(msp_multiaddress -> msp (msp_id));
diesel::joinable!(msp_multiaddress -> multiaddress (multiaddress_id));

diesel::allow_tables_to_appear_in_same_query!(
    bsp,
    bsp_file,
    bsp_multiaddress,
    bucket,
    file,
    file_peer_id,
    msp,
    msp_multiaddress,
    multiaddress,
    paymentstream,
    peer_id,
    service_state,
);
