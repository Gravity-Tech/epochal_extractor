table! {
    extracted_data (id) {
        id -> Uuid,
        base64bytes -> Varchar,
        block_id -> Timestamp,
        priority -> Int4,
        port -> Int4,
    }
}

table! {
    processed_solana_data_accounts (id) {
        id -> Int8,
        account_id -> Varchar,
        destination_txn_id -> Nullable<Varchar>,
    }
}

table! {
    poller_states (id) {
        id -> Int4,
        num -> Int8,
    }
}

allow_tables_to_appear_in_same_query!(
    extracted_data,
    poller_states,
    processed_solana_data_accounts,
);
