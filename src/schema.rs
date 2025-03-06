// @generated automatically by Diesel CLI.

diesel::table! {
    groups (id) {
        id -> Integer,
        name -> Text,
        hub_id -> Integer,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    groups_recipients (group_id, recipient_id) {
        group_id -> Integer,
        recipient_id -> Integer,
    }
}

diesel::table! {
    hubs (id) {
        id -> Integer,
        name -> Text,
        login -> Nullable<Text>,
        password -> Nullable<Text>,
        sender -> Nullable<Text>,
        server -> Nullable<Text>,
        port -> Nullable<Integer>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    recipients (id) {
        id -> Integer,
        name -> Text,
        email -> Text,
        hub_id -> Integer,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        email -> Text,
        password -> Text,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        hub_id -> Nullable<Integer>,
    }
}

diesel::joinable!(groups -> hubs (hub_id));
diesel::joinable!(groups_recipients -> groups (group_id));
diesel::joinable!(groups_recipients -> recipients (recipient_id));
diesel::joinable!(recipients -> hubs (hub_id));
diesel::joinable!(users -> hubs (hub_id));

diesel::allow_tables_to_appear_in_same_query!(
    groups,
    groups_recipients,
    hubs,
    recipients,
    users,
);
