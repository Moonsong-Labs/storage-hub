// @generated automatically by Diesel CLI.

diesel::table! {
    pending_transactions (account_id, nonce) {
        account_id -> Bytea,
        nonce -> Int4,
        hash -> Bytea,
        call_scale -> Bytea,
        state -> Text,
        creator_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
