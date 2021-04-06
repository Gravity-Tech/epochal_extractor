table! {
    extracted_data (id) {
        id -> Int4,
        base64bytes -> Varchar,
    }
}

table! {
    poller_states (id) {
        id -> Int4,
        num -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(
    extracted_data,
    poller_states,
);
