// @generated automatically by Diesel CLI.

diesel::table! {
    leader_info (id) {
        id -> Int4,
        metadata -> Jsonb,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    pending_transactions (account_id, nonce) {
        account_id -> Bytea,
        nonce -> Int8,
        hash -> Bytea,
        call_scale -> Nullable<Bytea>,
        extrinsic_scale -> Bytea,
        watched -> Bool,
        state -> Text,
        creator_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    leader_info,
    pending_transactions,
);
