// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Integer,
        email -> Text,
        password -> Text,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}
