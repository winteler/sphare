#![allow(dead_code)]

use activitypub_federation::fetch::object_id::ObjectId;
use leptos::serde_json;
use leptos::serde_json::json;
use url::Url;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use sphare_core_apub::community_moderator::{generate_moderators_url, GroupModerators};
use sphare_core_apub::group::{ApubSphere, Group};
use sphare_core_apub::person::ApubPerson;
use sphare_core_common::activity_pub::generate_rsa_keys_pem;

pub fn get_mock_server_url(mock_server: &MockServer) -> Url {
    Url::parse(&format!("http://localhost:{}", mock_server.address().port())).expect("Mock server uri should be valid")
}

pub async fn mock_apub_group(mock_server: &MockServer) -> Group {
    let mock_server_url = get_mock_server_url(mock_server);
    let group_json = json!({
        "id": format!("{mock_server_url}/c/tenforward"),
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
            "url": format!("{mock_server_url}/pictrs/image/waqyZwLAy4.webp")
        },
        "image": {
            "type": "Image",
            "url": format!("{mock_server_url}/pictrs/image/Wt8zoMcCmE.jpg")
        },
        "inbox": format!("{mock_server_url}/c/tenforward/inbox"),
        "followers": format!("{mock_server_url}/c/tenforward/followers"),
        "attributedTo": format!("{mock_server_url}/c/tenforward/moderators"),
        "featured": format!("{mock_server_url}/c/tenforward//featured"),
        "postingRestrictedToMods": false,
        "endpoints": {
            "sharedInbox": format!("{mock_server_url}/inbox")
        },
        "outbox": format!("{mock_server_url}/c/tenforward/outbox"),
        "publicKey": {
            "id": format!("{mock_server_url}/c/tenforward#main-key"),
            "owner": format!("{mock_server_url}/c/tenforward"),
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
                "id": format!("{mock_server_url}/c/tenforward/tag/news"),
                "preferredUsername": "news"
            }
        ],
        "published": "2019-06-02T16:43:50.799554Z",
        "updated": "2021-03-10T17:18:10.498868Z"
    });

    Mock::given(method("GET"))
        .and(path("/c/tenforward"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(group_json.to_string(), "application/activity+json")
        )
        .mount(mock_server)
        .await;

    serde_json::from_value(group_json).expect("Should deserialize group json")
}

pub async fn mock_apub_group_moderators(
    sphere_apub_id: &ObjectId<ApubSphere>,
    username_vec: &[&str],
    mock_server: &MockServer
) -> GroupModerators {
    let mock_server_url = get_mock_server_url(mock_server);
    let actor_id_vec: Vec<ObjectId<ApubPerson>> = username_vec
        .iter()
        .map(|name| mock_server_url.join(&format!("/actors/{}", name)).expect("Should join actor path").into())
        .collect();
    let group_moderator = GroupModerators {
        r#type: Default::default(),
        apub_id: generate_moderators_url(sphere_apub_id).expect("Should generate moderators url"),
        ordered_items: actor_id_vec,
    };
    let group_moderator_json = serde_json::to_string(&group_moderator).expect("Should serialize group moderators");

    Mock::given(method("GET"))
        .and(path(group_moderator.apub_id.to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(group_moderator_json.to_string(), "application/activity+json")
        )
        .mount(mock_server)
        .await;

    group_moderator
}


pub async fn mock_apub_persons(
    username_vec: &[&str],
    mock_server: &MockServer
) -> Vec<ObjectId<ApubPerson>> {
    let mock_server_url = get_mock_server_url(mock_server);
    let mut actor_ids = Vec::new();
    for username in username_vec {
        let actor_id: ObjectId<ApubPerson> = mock_server_url.join(&format!("/actors/{}", username)).expect("Should join actor path").into();
        let (pub_key_pem, _priv_key_pem) = generate_rsa_keys_pem().expect("Should get keys");

        let actor_json = json!({
            "id": actor_id.inner().to_string(),
            "type": "Person",
            "preferredUsername": username,
            "inbox": format!("{}/inbox", actor_id),
            "outbox": format!("{}/outbox", actor_id),
            "publicKey": {
                "id": format!("{}/#main-key", actor_id),
                "owner": actor_id,
                "publicKeyPem": pub_key_pem,
            },
        });

        Mock::given(method("GET"))
            .and(path(format!("/actors/{}", username)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(actor_json.to_string(), "application/activity+json")
            )
            .mount(mock_server)
            .await;
        actor_ids.push(actor_id.clone());
    }

    actor_ids
}