table! {
    collectors (id) {
        id -> Text,
        project -> Text,
        url -> Text,
    }
}

table! {
    latest_times (timestamp, collector_id, data_type) {
        timestamp -> Int8,
        collector_id -> Text,
        data_type -> Text,
        project -> Text,
        collector_url -> Text,
        item_url -> Text,
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
        file_info -> Jsonb,
    }
}

joinable!(items -> collectors (collector_id));

allow_tables_to_appear_in_same_query!(
    collectors,
    items,
);
