use sphare_core_content::embed::{verify_link_and_get_embed, EmbedType, Link, LinkType};

#[tokio::test]
async fn test_verify_link_and_get_embed() {
    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::None,
            "this is not an url",
        ).await,
        (Link::default(), None),
    );

    let link_url = String::from("https://www.test.com/example");
    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::None,
            &link_url,
        ).await,
        (Link::new(LinkType::Link, Some(link_url.clone()), None, None), None),
    );

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Embed,
            &link_url,
        ).await,
        (Link::new(LinkType::Link, Some(link_url), None, None), None),
    );

    let image_url = String::from("https://www.test.com/image.jpg");
    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::None,
            &image_url,
        ).await,
        (Link::new(LinkType::Image, Some(image_url.clone()), None, None), None),
    );

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Link,
            &image_url,
        ).await,
        (Link::new(LinkType::Link, Some(image_url.clone()), None, None), None),
    );

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Embed,
            &image_url,
        ).await,
        (Link::new(LinkType::Image, Some(image_url), None, None), None),
    );

    let video_url = String::from("https://www.test.com/video.mp4");
    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::None,
            &video_url,
        ).await,
        (Link::new(LinkType::Video, Some(video_url.clone()), None, None), None),
    );

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Link,
            &video_url,
        ).await,
        (Link::new(LinkType::Link, Some(video_url.clone()), None, None), None),
    );

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Embed,
            &video_url,
        ).await,
        (Link::new(LinkType::Video, Some(video_url), None, None), None),
    );
}

#[tokio::test]
async fn test_verify_link_and_get_embed_giphy() {
    let giphy_url = String::from("https://giphy.com/gifs/justin-raccoon-pedro-tHIRLHtNwxpjIFqPdV");
    let (giphy_link, giphy_title) = verify_link_and_get_embed(
        EmbedType::None,
        &giphy_url,
    ).await;
    assert_eq!(giphy_link.link_type, LinkType::Image);
    assert!(giphy_link.link_url.is_some());
    assert!(giphy_link.link_embed.is_none());
    assert!(giphy_link.link_thumbnail_url.is_none());
    assert!(giphy_title.is_some());

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Link,
            &giphy_url,
        ).await,
        (Link::new(LinkType::Link, Some(giphy_url.clone()), None, None), None),
    );

    let giphy_url = String::from("https://giphy.com/gifs/justin-raccoon-pedro-tHIRLHtNwxpjIFqPdV");
    let (giphy_link, giphy_title) = verify_link_and_get_embed(
        EmbedType::Embed,
        &giphy_url,
    ).await;
    assert_eq!(giphy_link.link_type, LinkType::Image);
    assert!(giphy_link.link_url.is_some());
    assert!(giphy_link.link_embed.is_none());
    assert!(giphy_link.link_thumbnail_url.is_none());
    assert!(giphy_title.is_some());
}

#[tokio::test]
async fn test_verify_link_and_get_embed_youtube() {
    let youtube_url = String::from("https://www.youtube.com/watch?v=4nUZtFL7jLs");
    let (youtube_link, youtube_title) = verify_link_and_get_embed(
        EmbedType::None,
        &youtube_url,
    ).await;
    assert_eq!(youtube_link.link_type, LinkType::Video);
    assert!(youtube_link.link_url.is_some());
    assert!(youtube_link.link_embed.is_some());
    assert!(youtube_link.link_thumbnail_url.is_some());
    assert!(youtube_title.is_some());

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Link,
            &youtube_url,
        ).await,
        (Link::new(LinkType::Link, Some(youtube_url.clone()), None, None), None),
    );
    
    let (youtube_link, youtube_title) = verify_link_and_get_embed(
        EmbedType::Embed,
        &youtube_url,
    ).await;
    assert_eq!(youtube_link.link_type, LinkType::Video);
    assert!(youtube_link.link_url.is_some());
    assert!(youtube_link.link_embed.is_some());
    assert!(youtube_link.link_thumbnail_url.is_some());
    assert!(youtube_title.is_some());
}

#[tokio::test]
async fn test_verify_link_and_get_embed_bluesky() {
    let bluesky_url = String::from("https://bsky.app/profile/gameofroles.bsky.social/post/3lfppr5oo722e");
    let (bluesky_link, bluesky_title) = verify_link_and_get_embed(
        EmbedType::None,
        &bluesky_url,
    ).await;
    assert_eq!(bluesky_link.link_type, LinkType::Rich);
    assert!(bluesky_link.link_url.is_some());
    assert!(bluesky_link.link_embed.is_some());
    assert!(bluesky_link.link_thumbnail_url.is_none());
    assert!(bluesky_title.is_none());

    assert_eq!(
        verify_link_and_get_embed(
            EmbedType::Link,
            &bluesky_url,
        ).await,
        (Link::new(LinkType::Link, Some(bluesky_url.clone()), None, None), None),
    );

    let (bluesky_link, bluesky_title) = verify_link_and_get_embed(
        EmbedType::Embed,
        &bluesky_url,
    ).await;
    assert_eq!(bluesky_link.link_type, LinkType::Rich);
    assert!(bluesky_link.link_url.is_some());
    assert!(bluesky_link.link_embed.is_some());
    assert!(bluesky_link.link_thumbnail_url.is_none());
    assert!(bluesky_title.is_none());
}