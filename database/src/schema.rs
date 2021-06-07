table! {
    extracted_data (id) {
        id -> Uuid,
        base64bytes -> Varchar,
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
);
