// @generated automatically by Diesel CLI.

diesel::table! {
    bsp (id) {
        id -> Int8,
        account -> Varchar,
        capacity -> Numeric,
        stake -> Numeric,
        last_tick_proven -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        onchain_bsp_id -> Varchar,
        merkle_root -> Bytea,
    }
}

diesel::table! {
    bsp_file (bsp_id, file_id) {
        bsp_id -> Int8,
        file_id -> Int8,
    }
}

diesel::table! {
    bsp_multiaddress (bsp_id, multiaddress_id) {
        bsp_id -> Int8,
        multiaddress_id -> Int8,
    }
}

diesel::table! {
    bucket (id) {
        id -> Int8,
        msp_id -> Nullable<Int8>,
        account -> Varchar,
        onchain_bucket_id -> Bytea,
        name -> Bytea,
        collection_id -> Nullable<Varchar>,
        private -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        merkle_root -> Bytea,
    }
}

diesel::table! {
    file (id) {
        id -> Int8,
        account -> Bytea,
        file_key -> Bytea,
        bucket_id -> Int8,
        onchain_bucket_id -> Bytea,
        location -> Bytea,
        fingerprint -> Bytea,
        size -> Int8,
        step -> Int4,
        deletion_status -> Nullable<Int4>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    file_peer_id (file_id, peer_id) {
        file_id -> Int8,
        peer_id -> Int8,
    }
}

diesel::table! {
    msp (id) {
        id -> Int8,
        account -> Varchar,
        capacity -> Numeric,
        value_prop -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        onchain_msp_id -> Varchar,
    }
}

diesel::table! {
    msp_file (msp_id, file_id) {
        msp_id -> Int8,
        file_id -> Int8,
    }
}

diesel::table! {
    msp_multiaddress (msp_id, multiaddress_id) {
        msp_id -> Int8,
        multiaddress_id -> Int8,
    }
}

diesel::table! {
    multiaddress (id) {
        id -> Int8,
        address -> Bytea,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    paymentstream (id) {
        id -> Int8,
        account -> Varchar,
        provider -> Varchar,
        total_amount_paid -> Numeric,
        last_tick_charged -> Int8,
        charged_at_tick -> Int8,
    }
}

diesel::table! {
    peer_id (id) {
        id -> Int8,
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

diesel::joinable!(bsp_file -> bsp (bsp_id));
diesel::joinable!(bsp_file -> file (file_id));
diesel::joinable!(bsp_multiaddress -> bsp (bsp_id));
diesel::joinable!(bsp_multiaddress -> multiaddress (multiaddress_id));
diesel::joinable!(bucket -> msp (msp_id));
diesel::joinable!(file_peer_id -> file (file_id));
diesel::joinable!(file_peer_id -> peer_id (peer_id));
diesel::joinable!(msp_file -> file (file_id));
diesel::joinable!(msp_file -> msp (msp_id));
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
    msp_file,
    msp_multiaddress,
    multiaddress,
    paymentstream,
    peer_id,
    service_state,
);
