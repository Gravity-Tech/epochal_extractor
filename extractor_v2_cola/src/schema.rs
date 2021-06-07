table! {
    achievements (id) {
        id -> Int4,
        name -> Varchar,
        description -> Varchar,
        icon -> Varchar,
    }
}

table! {
    achievements_to_users (id) {
        id -> Int4,
        user_id -> Nullable<Int4>,
        achievement_id -> Nullable<Int4>,
    }
}

table! {
    gton_price (id) {
        id -> Int4,
        price -> Float8,
        market_time -> Timestamp,
    }
}

table! {
    pollers_data (id) {
        id -> Int4,
        block_id -> Int8,
        poller_id -> Int4,
    }
}

table! {
    total_values_for_users (id) {
        id -> Int4,
        user_id -> Int4,
        sender_id -> Int4,
        amount -> Int8,
    }
}

table! {
    users (id) {
        id -> Int4,
        address -> Varchar,
        twitter_account -> Nullable<Varchar>,
    }
}

table! {
    value_senders (id) {
        id -> Int4,
        address -> Varchar,
        name -> Varchar,
        amount -> Int8,
    }
}

table! {
    voters (id) {
        id -> Int4,
        round_id -> Int4,
        user_address -> Varchar,
        vote_times -> Int4,
    }
}

joinable!(achievements_to_users -> achievements (achievement_id));
joinable!(achievements_to_users -> users (user_id));
joinable!(total_values_for_users -> users (user_id));
joinable!(total_values_for_users -> value_senders (sender_id));

allow_tables_to_appear_in_same_query!(
    achievements,
    achievements_to_users,
    gton_price,
    pollers_data,
    total_values_for_users,
    users,
    value_senders,
    voters,
);
