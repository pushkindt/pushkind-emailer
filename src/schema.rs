// @generated automatically by Diesel CLI.

diesel::table! {
    email_recipients (id) {
        id -> Integer,
        email_id -> Integer,
        address -> Text,
        opened -> Bool,
        updated_at -> Timestamp,
        is_sent -> Bool,
        replied -> Bool,
    }
}

diesel::table! {
    emails (id) {
        id -> Integer,
        user_id -> Integer,
        message -> Text,
        created_at -> Timestamp,
        is_sent -> Bool,
        subject -> Nullable<Text>,
    }
}

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
        smtp_server -> Nullable<Text>,
        smtp_port -> Nullable<Integer>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        imap_server -> Nullable<Text>,
        imap_port -> Nullable<Integer>,
    }
}

diesel::table! {
    recipient_fields (recipient_id, field) {
        recipient_id -> Integer,
        field -> Text,
        value -> Text,
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
        unsubscribed_at -> Nullable<Timestamp>,
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

diesel::joinable!(email_recipients -> emails (email_id));
diesel::joinable!(emails -> users (user_id));
diesel::joinable!(groups -> hubs (hub_id));
diesel::joinable!(groups_recipients -> groups (group_id));
diesel::joinable!(groups_recipients -> recipients (recipient_id));
diesel::joinable!(recipient_fields -> recipients (recipient_id));
diesel::joinable!(recipients -> hubs (hub_id));
diesel::joinable!(users -> hubs (hub_id));

diesel::allow_tables_to_appear_in_same_query!(
    email_recipients,
    emails,
    groups,
    groups_recipients,
    hubs,
    recipient_fields,
    recipients,
    users,
);
