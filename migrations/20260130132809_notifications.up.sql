CREATE TABLE notifications (
    notification_id BIGSERIAL PRIMARY KEY,
    sphere_id BIGINT NOT NULL REFERENCES spheres(sphere_id),
    satellite_id BIGINT REFERENCES satellites(satellite_id),
    post_id BIGINT NOT NULL REFERENCES posts(post_id),
    comment_id BIGINT REFERENCES comments(comment_id),
    user_id BIGINT NOT NULL REFERENCES users(user_id),
    trigger_person_id BIGINT NOT NULL REFERENCES persons(person_id),
    notification_type SMALLINT NOT NULL CHECK (notification_type IN (0, 1, 2)),
    is_read BOOL NOT NULL DEFAULT FALSE,
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications
    ON notifications (user_id, create_timestamp DESC);
