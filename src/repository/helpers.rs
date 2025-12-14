use std::collections::HashMap;

use diesel::prelude::*;
use diesel::query_dsl::methods::{LimitDsl, OffsetDsl};

use pushkind_common::db::DbConnection;
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::recipient::Recipient as DomainRecipient;
use crate::models::group::GroupRecipient;
use crate::models::recipient::{Recipient as DbRecipient, RecipientField};

pub(super) fn apply_pagination<T>(mut query: T, pagination: Option<&Pagination>) -> T
where
    T: OffsetDsl<Output = T> + LimitDsl<Output = T>,
{
    if let Some(pagination) = pagination {
        let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
        let limit = pagination.per_page as i64;
        query = query.offset(offset).limit(limit);
    }

    query
}

pub(super) fn hydrate_recipients(
    conn: &mut DbConnection,
    hub_id: i32,
    db_recipients: Vec<DbRecipient>,
) -> RepositoryResult<Vec<DomainRecipient>> {
    use crate::schema::unsubscribes;

    if db_recipients.is_empty() {
        return Ok(Vec::new());
    }

    let recipient_emails: Vec<String> = db_recipients
        .iter()
        .map(|recipient| recipient.email.clone())
        .collect();

    let unsubscribed_lookup = if recipient_emails.is_empty() {
        HashMap::new()
    } else {
        unsubscribes::table
            .filter(unsubscribes::hub_id.eq(hub_id))
            .filter(unsubscribes::email.eq_any(&recipient_emails))
            .select((unsubscribes::email, unsubscribes::created_at))
            .load::<(String, chrono::NaiveDateTime)>(conn)?
            .into_iter()
            .collect::<HashMap<_, _>>()
    };

    let db_fields = RecipientField::belonging_to(&db_recipients)
        .select(RecipientField::as_select())
        .load::<RecipientField>(conn)?
        .grouped_by(&db_recipients);

    let db_group_recipients = GroupRecipient::belonging_to(&db_recipients)
        .select(GroupRecipient::as_select())
        .load::<GroupRecipient>(conn)?;

    let mut recipient_id_to_group_ids: HashMap<i32, Vec<i32>> = HashMap::new();
    for relation in db_group_recipients {
        recipient_id_to_group_ids
            .entry(relation.recipient_id)
            .or_default()
            .push(relation.group_id);
    }

    let recipients = db_recipients
        .into_iter()
        .zip(db_fields)
        .map(|(r, fields)| {
            let unsubscribed_at = unsubscribed_lookup.get(&r.email).copied();
            let groups = recipient_id_to_group_ids.remove(&r.id).unwrap_or_default();
            DomainRecipient {
                unsubscribed_at,
                id: r.id,
                name: r.name,
                email: r.email,
                hub_id: r.hub_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
                fields: fields
                    .into_iter()
                    .map(|f| (f.field, f.value))
                    .collect::<HashMap<_, _>>(),
                groups,
            }
        })
        .collect();

    Ok(recipients)
}
