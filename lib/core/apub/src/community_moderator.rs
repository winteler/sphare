use activitypub_federation::config::Data;
use activitypub_federation::fetch::object_id::ObjectId;
use activitypub_federation::kinds::collection::OrderedCollectionType;
use activitypub_federation::protocol::verification::verify_domains_match;
use activitypub_federation::traits::Collection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use url::Url;

use sphare_core_common::activity_pub::ApubHelper;
use sphare_core_common::errors::AppError;
use sphare_core_user::role::PermissionLevel;
use sphare_core_user::user::ssr::get_admin_function_user;

use crate::group::ApubSphere;
use crate::person::{ApubPerson, DbPerson};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupModerators {
    pub(crate) r#type: OrderedCollectionType,
    pub(crate) apub_id: Url,
    pub(crate) ordered_items: Vec<ObjectId<ApubPerson>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ApubCommunityModerators(());

#[async_trait::async_trait]
impl Collection for ApubCommunityModerators {
    type Owner = ApubSphere;
    type DataType = ApubHelper;
    type Kind = GroupModerators;
    type Error = AppError;

    async fn read_local(owner: &Self::Owner, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let moderator_vec = get_sphere_moderators(owner, data.get_db_pool()).await?;
        let ordered_items = moderator_vec
            .into_iter()
            .map(|m| m.apub_id)
            .collect();
        Ok(GroupModerators {
            r#type: OrderedCollectionType::OrderedCollection,
            apub_id: generate_moderators_url(&owner.apub_id)?,
            ordered_items,
        })
    }

    async fn verify(
        group_moderators: &GroupModerators,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(&group_moderators.apub_id, expected_domain)?;
        Ok(())
    }

    async fn from_json(
        apub: Self::Kind,
        owner: &Self::Owner,
        data: &Data<Self::DataType>,
    ) -> Result<ApubCommunityModerators, Self::Error> {
        handle_community_moderators(&apub.ordered_items, owner, data).await?;

        // This return value is unused, so just set an empty vec
        Ok(ApubCommunityModerators(()))
    }
}

fn generate_moderators_url(sphere_apub_id: &ObjectId<ApubSphere>) -> Result<Url, AppError> {
    let url = sphere_apub_id.inner().clone().join("/moderators")?;
    Ok(url)
}

pub async fn get_sphere_moderators(apub_sphere: &ApubSphere, db_pool: &PgPool) -> Result<Vec<ApubPerson>, AppError> {
    let sphere_mod_vec = sqlx::query_as!(
            DbPerson,
            "SELECT p.*, u.private_key FROM persons p
            JOIN users u ON u.person_id = p.person_id
            JOIN user_sphere_roles r ON r.person_id = p.person_id
            JOIN spheres s ON s.sphere_id = r.sphere_id
            WHERE
                s.sphere_apub_id = $1 AND
                r.permission_level != 'None' AND
                r.delete_timestamp IS NULL",
            apub_sphere.apub_id.inner().to_string(),
        )
            .fetch_all(db_pool)
            .await?;

    let sphere_mod_vec = sphere_mod_vec
        .into_iter()
        .filter_map(|m| ApubPerson::try_from(m).ok())
        .collect();

    Ok(sphere_mod_vec)
}

pub async fn handle_community_moderators(
    new_mod_vec: &Vec<ObjectId<ApubPerson>>,
    community: &ApubSphere,
    context: &Data<ApubHelper>,
) -> Result<(), AppError> {
    // Fetch moderators
    for new_mod in new_mod_vec {
        let fetch_result = new_mod.dereference(context).await;
        if let Err(e) = fetch_result {
            log::warn!("Failed to dereference community moderator {}: {}", new_mod.inner(), e);
        }
    }
    let functional_user = get_admin_function_user(context.get_db_pool()).await?;

    let person_apub_id_vec: Vec<String> = new_mod_vec.iter().map(|i| i.inner().to_string()).collect();

    // Upsert mods and delete those no longer presents
    sqlx::query!(
        r#"
        WITH user_ids AS (
            SELECT p.person_id
            FROM UNNEST($1::text[]) AS t(person_apub_id)
            JOIN persons p ON p.actor_id = t.person_apub_id
        ),
        remote_sphere AS (
            SELECT *
            FROM spheres
            WHERE sphere_apub_id = $2
        ),
        upserted_mods AS (
            INSERT INTO user_sphere_roles (person_id, sphere_id, permission_level, grantor_id)
            SELECT u.person_id, s.sphere_id, $3, $4
            FROM user_ids u, remote_sphere s
            ON CONFLICT (sphere_id, person_id) WHERE delete_timestamp IS NULL DO UPDATE
            SET permission_level = EXCLUDED.permission_level,
                grantor_id = EXCLUDED.grantor_id
            RETURNING person_id
        )
        DELETE FROM user_sphere_roles r
        WHERE r.sphere_id = (SELECT sphere_id FROM remote_sphere) AND NOT (r.person_id = ANY(SELECT person_id FROM user_ids))
        "#,
        &person_apub_id_vec,
        community.apub_id.inner().to_string(),
        PermissionLevel::Manage.to_string(),
        functional_user.user_id,
    )
        .execute(context.get_db_pool())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::community_moderator::generate_moderators_url;
    use url::Url;

    #[test]
    fn test_generate_moderators_url() {
        let sphere_url = Url::parse("https://www.sphare.space/c/SomeSphere").expect("Should be valid group url");
        let sphere_apub_id = sphere_url.clone().into();
        assert_eq!(generate_moderators_url(&sphere_apub_id), Ok(sphere_url.join("/moderators").expect("Should be valid moderators url")));
    }
}