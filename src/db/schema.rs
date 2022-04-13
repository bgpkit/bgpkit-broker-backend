table! {
    collectors (id) {
        id -> Text,
        project -> Text,
        url -> Text,
    }
}

table! {
    items (url) {
        ts_start -> Int8,
        ts_end -> Int8,
        collector_id -> Text,
        data_type -> Text,
        url -> Text,
        file_size -> Int8,
    }
}

joinable!(items -> collectors (collector_id));

allow_tables_to_appear_in_same_query!(
    collectors,
    items,
);
