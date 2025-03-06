// @generated automatically by Diesel CLI.

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
    users (id) {
        id -> Integer,
        email -> Text,
        password -> Text,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        hub_id -> Nullable<Integer>,
    }
}

diesel::joinable!(users -> hubs (hub_id));

diesel::allow_tables_to_appear_in_same_query!(
    hubs,
    users,
);
