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
        reply -> Nullable<Text>,
        name -> Text,
        fields -> Text,
    }
}

diesel::table! {
    emails (id) {
        id -> Integer,
        message -> Text,
        created_at -> Timestamp,
        is_sent -> Bool,
        subject -> Nullable<Text>,
        attachment -> Nullable<Binary>,
        attachment_name -> Nullable<Text>,
        attachment_mime -> Nullable<Text>,
        num_sent -> Integer,
        num_opened -> Integer,
        num_replied -> Integer,
        hub_id -> Integer,
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
        login -> Nullable<Text>,
        password -> Nullable<Text>,
        sender -> Nullable<Text>,
        smtp_server -> Nullable<Text>,
        smtp_port -> Nullable<Integer>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        imap_server -> Nullable<Text>,
        imap_port -> Nullable<Integer>,
        email_template -> Nullable<Text>,
        imap_last_uid -> Integer,
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
    recipient_fts (rowid) {
        rowid -> Integer,
        name -> Nullable<Binary>,
        email -> Nullable<Binary>,
        fields -> Nullable<Binary>,
        #[sql_name = "recipient_fts"]
        recipient_fts_col -> Nullable<Binary>,
        rank -> Nullable<Binary>,
    }
}

diesel::table! {
    recipient_fts_config (k) {
        k -> Binary,
        v -> Nullable<Binary>,
    }
}

diesel::table! {
    recipient_fts_data (id) {
        id -> Nullable<Integer>,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    recipient_fts_docsize (id) {
        id -> Nullable<Integer>,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    recipient_fts_idx (segid, term) {
        segid -> Binary,
        term -> Binary,
        pgno -> Nullable<Binary>,
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
        fields -> Nullable<Text>,
    }
}

diesel::table! {
    unsubscribes (email, hub_id) {
        email -> Text,
        hub_id -> Integer,
        reason -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(email_recipients -> emails (email_id));
diesel::joinable!(emails -> hubs (hub_id));
diesel::joinable!(groups -> hubs (hub_id));
diesel::joinable!(groups_recipients -> groups (group_id));
diesel::joinable!(groups_recipients -> recipients (recipient_id));
diesel::joinable!(recipient_fields -> recipients (recipient_id));
diesel::joinable!(recipients -> hubs (hub_id));

diesel::allow_tables_to_appear_in_same_query!(
    email_recipients,
    emails,
    groups,
    groups_recipients,
    hubs,
    recipient_fields,
    recipient_fts,
    recipient_fts_config,
    recipient_fts_data,
    recipient_fts_docsize,
    recipient_fts_idx,
    recipients,
    unsubscribes,
);
