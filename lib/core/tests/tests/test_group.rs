use leptos::serde_json;
use url::Url;
use sphare_core_apub::group::{get_sphere_by_apub_id, insert_or_update_sphere};
use sphare_core_sphere::sphere::ssr::create_sphere;
use crate::common::{create_test_user, get_db_pool};

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_get_sphere_by_apub_id() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let sphere = create_sphere("Activity", "Pub", false, &user, &db_pool).await.expect("Should create sphere");
    let sphere_apub_id = Url::parse(&sphere.sphere_apub_id).expect("Should parse sphere URL");
    let result_sphere = get_sphere_by_apub_id(&sphere_apub_id, &db_pool).await.expect("Should get sphere by apub");
    assert_eq!(result_sphere, Some(sphere));

    let unknown_apub_id = Url::parse("https://unknown.apub/id").expect("Should create unknown URL");
    let result_sphere = get_sphere_by_apub_id(&unknown_apub_id, &db_pool).await.expect("Should get sphere by apub");
    assert_eq!(result_sphere, None);
}

#[tokio::test]
async fn test_insert_or_update_sphere() {
    let db_pool = get_db_pool().await;

    let group_id_str = "https://enterprise.lemmy.ml/c/tenforward";
    let group_id = Url::parse(group_id_str).expect("Should be valid url");

    assert_eq!(get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option"), None);
    let group_json = r#"{
        "id": "https://enterprise.lemmy.ml/c/tenforward",
        "type": "Group",
        "preferredUsername": "tenforward",
        "name": "Ten Forward",
        "description": "A description of ten forward.",
        "summary": "<p>Lounge and recreation facility</p>\n<hr />\n<p>Welcome to the Enterprise!.</p>\n",
        "source": {
        "content": "Lounge and recreation facility\n\n---\n\nWelcome to the Enterprise!",
        "mediaType": "text/markdown"
        },
        "mediaType": "text/html",
        "sensitive": false,
        "icon": {
        "type": "Image",
        "url": "https://enterprise.lemmy.ml/pictrs/image/waqyZwLAy4.webp"
        },
        "image": {
        "type": "Image",
        "url": "https://enterprise.lemmy.ml/pictrs/image/Wt8zoMcCmE.jpg"
        },
        "inbox": "https://enterprise.lemmy.ml/c/tenforward/inbox",
        "followers": "https://enterprise.lemmy.ml/c/tenforward/followers",
        "attributedTo": "https://enterprise.lemmy.ml/c/tenforward/moderators",
        "featured": "https://enterprise.lemmy.ml/c/tenforward//featured",
        "postingRestrictedToMods": false,
        "endpoints": {
        "sharedInbox": "https://enterprise.lemmy.ml/inbox"
        },
        "outbox": "https://enterprise.lemmy.ml/c/tenforward/outbox",
        "publicKey": {
        "id": "https://enterprise.lemmy.ml/c/tenforward#main-key",
        "owner": "https://enterprise.lemmy.ml/c/tenforward",
        "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzRjKTNtvDCmugplwEh+g\nx1bhKm6BHUZfXfpscgMMm7tXFswSDzUQirMgfkxa9ubfr1PDFKffA2vQ9x6CyuO/\n70xTafdOHyV1tSqzgKz0ZvFZ/VCOo6qy1mYWVkrtBm/fKzM+87MdkKYB/zI4VyEJ\nLfLQgjwxBAEYUH3CBG71U0gO0TwbimWNN0vqlfp0QfThNe1WYObF88ZVzMLgFbr7\nRHBItZjlZ/d8foPDidlIR3l2dJjy0EsD8F9JM340jtX7LXqFmU4j1AQKNHTDLnUF\nwYVhzuQGNJ504l5LZkFG54XfIFT7dx2QwuuM9bSnfPv/98RYrq1Si6tCkxEt1cVe\n4wIDAQAB\n-----END PUBLIC KEY-----\n"
        },
        "language": [
        {
          "identifier": "fr",
          "name": "Français"
        },
        {
          "identifier": "de",
          "name": "Deutsch"
        }
        ],
        "tag": [
        {
          "type": "CommunityPostTag",
          "id": "https://enterprise.lemmy.ml/c/tenforward/tag/news",
          "preferredUsername": "news"
        }
        ],
        "published": "2019-06-02T16:43:50.799554Z",
        "updated": "2021-03-10T17:18:10.498868Z"
    }"#;
    let group = serde_json::from_str(group_json).expect("Should deserialize Group");

    let sphere = insert_or_update_sphere(group, &db_pool).await.expect("Should get Sphere");
    assert_eq!(sphere.sphere_name, "tenforward");
    assert_eq!(sphere.sphere_apub_id, "https://enterprise.lemmy.ml/c/tenforward");
    assert_eq!(sphere.description, "A description of ten forward.");
    assert_eq!(sphere.is_nsfw, false);
    assert_eq!(sphere.inbox, "https://enterprise.lemmy.ml/inbox");
    assert_eq!(sphere.public_key, "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzRjKTNtvDCmugplwEh+g\nx1bhKm6BHUZfXfpscgMMm7tXFswSDzUQirMgfkxa9ubfr1PDFKffA2vQ9x6CyuO/\n70xTafdOHyV1tSqzgKz0ZvFZ/VCOo6qy1mYWVkrtBm/fKzM+87MdkKYB/zI4VyEJ\nLfLQgjwxBAEYUH3CBG71U0gO0TwbimWNN0vqlfp0QfThNe1WYObF88ZVzMLgFbr7\nRHBItZjlZ/d8foPDidlIR3l2dJjy0EsD8F9JM340jtX7LXqFmU4j1AQKNHTDLnUF\nwYVhzuQGNJ504l5LZkFG54XfIFT7dx2QwuuM9bSnfPv/98RYrq1Si6tCkxEt1cVe\n4wIDAQAB\n-----END PUBLIC KEY-----\n");

    let result_sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(result_sphere, sphere);

    let updated_group_json = r#"{
        "id": "https://enterprise.lemmy.ml/c/tenforward",
        "type": "Group",
        "preferredUsername": "tenforward",
        "name": "Ten Forward",
        "description": "An updated description of ten forward.",
        "summary": "<p>Lounge and recreation facility</p>\n<hr />\n<p>Welcome to the Enterprise!.</p>\n",
        "source": {
        "content": "Lounge and recreation facility\n\n---\n\nWelcome to the Enterprise!",
        "mediaType": "text/markdown"
        },
        "mediaType": "text/html",
        "sensitive": false,
        "icon": {
        "type": "Image",
        "url": "https://enterprise.lemmy.ml/pictrs/image/waqyZwLAy4.webp"
        },
        "image": {
        "type": "Image",
        "url": "https://enterprise.lemmy.ml/pictrs/image/Wt8zoMcCmE.jpg"
        },
        "inbox": "https://enterprise.lemmy.ml/c/tenforward/inbox-updated",
        "followers": "https://enterprise.lemmy.ml/c/tenforward/followers",
        "attributedTo": "https://enterprise.lemmy.ml/c/tenforward/moderators",
        "featured": "https://enterprise.lemmy.ml/c/tenforward//featured",
        "postingRestrictedToMods": false,
        "endpoints": {
        "sharedInbox": "https://enterprise.lemmy.ml/inbox"
        },
        "outbox": "https://enterprise.lemmy.ml/c/tenforward/outbox",
        "publicKey": {
        "id": "https://enterprise.lemmy.ml/c/tenforward#main-key",
        "owner": "https://enterprise.lemmy.ml/c/tenforward",
        "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIZZZZZZZZw0BAQEFAAOCAQ8AMIIBCgKCAQEAzRjKTNtvDCmugplwEh+g\nx1bhKm6BHUZfXfpscgMMm7tXFswSDzUQirMgfkxa9ubfr1PDFKffA2vQ9x6CyuO/\n70xTafdOHyV1tSqzgKz0ZvFZ/VCOo6qy1mYWVkrtBm/fKzM+87MdkKYB/zI4VyEJ\nLfLQgjwxBAEYUH3CBG71U0gO0TwbimWNN0vqlfp0QfThNe1WYObF88ZVzMLgFbr7\nRHBItZjlZ/d8foPDidlIR3l2dJjy0EsD8F9JM340jtX7LXqFmU4j1AQKNHTDLnUF\nwYVhzuQGNJ504l5LZkFG54XfIFT7dx2QwuuM9bSnfPv/98RYrq1Si6tCkxEt1cVe\n4wIDAQAB\n-----END PUBLIC KEY-----\n"
        },
        "language": [
        {
          "identifier": "fr",
          "name": "Français"
        },
        {
          "identifier": "de",
          "name": "Deutsch"
        }
        ],
        "tag": [
        {
          "type": "CommunityPostTag",
          "id": "https://enterprise.lemmy.ml/c/tenforward/tag/news",
          "preferredUsername": "news"
        }
        ],
        "published": "2019-06-02T16:43:50.799554Z",
        "updated": "2021-03-10T17:18:10.498868Z"
    }"#;
    let updated_group = serde_json::from_str(updated_group_json).expect("Should deserialize Group");

    let updated_sphere = insert_or_update_sphere(updated_group, &db_pool).await.expect("Should get sphere");
    assert_eq!(updated_sphere.sphere_name, "tenforward");
    assert_eq!(updated_sphere.sphere_apub_id, "https://enterprise.lemmy.ml/c/tenforward");
    assert_eq!(updated_sphere.description, "An updated description of ten forward.");
    assert_eq!(updated_sphere.is_nsfw, false);
    assert_eq!(updated_sphere.inbox, "https://enterprise.lemmy.ml/inbox");
    assert_eq!(updated_sphere.public_key, "-----BEGIN PUBLIC KEY-----\nMIZZZZZZZZw0BAQEFAAOCAQ8AMIIBCgKCAQEAzRjKTNtvDCmugplwEh+g\nx1bhKm6BHUZfXfpscgMMm7tXFswSDzUQirMgfkxa9ubfr1PDFKffA2vQ9x6CyuO/\n70xTafdOHyV1tSqzgKz0ZvFZ/VCOo6qy1mYWVkrtBm/fKzM+87MdkKYB/zI4VyEJ\nLfLQgjwxBAEYUH3CBG71U0gO0TwbimWNN0vqlfp0QfThNe1WYObF88ZVzMLgFbr7\nRHBItZjlZ/d8foPDidlIR3l2dJjy0EsD8F9JM340jtX7LXqFmU4j1AQKNHTDLnUF\nwYVhzuQGNJ504l5LZkFG54XfIFT7dx2QwuuM9bSnfPv/98RYrq1Si6tCkxEt1cVe\n4wIDAQAB\n-----END PUBLIC KEY-----\n");


    let result_updated_sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(result_updated_sphere, updated_sphere);
}