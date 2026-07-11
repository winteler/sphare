CREATE TABLE activity_pub_updates (
    update_id BIGSERIAL PRIMARY KEY,
    content_type TEXT NOT NULL CHECK (content_type IN ('Post', 'Comment', 'Vote', 'Moderation')),
    content_id BIGINT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);