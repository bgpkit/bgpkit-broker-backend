table! {
    collectors (id) {
        id -> Text,
        project -> Text,
        url -> Text,
    }
}

table! {
    items (url) {
        ts_start -> Timestamp,
        ts_end -> Timestamp,
        collector_id -> Text,
        data_type -> Text,
        url -> Text,
        rough_size -> Int8,
        exact_size -> Int8,
    }
}

joinable!(items -> collectors (collector_id));

allow_tables_to_appear_in_same_query!(
    collectors,
    items,
);
