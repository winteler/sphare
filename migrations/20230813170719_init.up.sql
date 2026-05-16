CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE OR REPLACE FUNCTION format_for_search(text) RETURNS text
LANGUAGE SQL
IMMUTABLE
RETURNS NULL ON NULL INPUT
AS $$
    SELECT LOWER(
       REGEXP_REPLACE(
           REGEXP_REPLACE($1, '([a-z])([A-Z])', '\1|\2', 'g'),
           '[-_]', ' ', 'g'
       )
   );
$$;

CREATE OR REPLACE FUNCTION normalize_sphere_name(text) RETURNS text
LANGUAGE SQL
IMMUTABLE
RETURNS NULL ON NULL INPUT
AS $$
    SELECT REPLACE(LOWER($1), '-', '_');
$$;


CREATE OR REPLACE FUNCTION score_mapping(score numeric)
    RETURNS numeric
    LANGUAGE sql
    IMMUTABLE
AS $$
    SELECT CASE
       WHEN $1 >= 0
           THEN $1 + 1
       ELSE
           1 / (1 - $1)
    END;
$$;

CREATE TABLE users (
    user_id BIGSERIAL PRIMARY KEY,
    oidc_id TEXT UNIQUE NOT NULL,
    username TEXT NOT NULL CHECK (LENGTH(username) <= 30),
    email TEXT NOT NULL,
    is_nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    admin_role TEXT NOT NULL DEFAULT 'None' CHECK (admin_role IN ('None', 'Moderator', 'Admin')),
    days_hide_spoiler INT CHECK (days_hide_spoiler > 0),
    show_nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_timestamp TIMESTAMPTZ,
    UNIQUE (user_id, username)
);

CREATE UNIQUE INDEX idx_unique_username ON users (username)
    WHERE users.delete_timestamp IS NULL;
CREATE UNIQUE INDEX idx_unique_email ON users (email)
    WHERE users.delete_timestamp IS NULL;

CREATE TABLE spheres (
    sphere_id BIGSERIAL PRIMARY KEY,
    sphere_name TEXT UNIQUE NOT NULL CHECK (LENGTH(sphere_name) <= 20),
    normalized_sphere_name TEXT UNIQUE NOT NULL GENERATED ALWAYS AS (
        normalize_sphere_name(sphere_name)
    ) STORED,
    search_sphere_name TEXT UNIQUE NOT NULL GENERATED ALWAYS AS (
        format_for_search(sphere_name)
    ) STORED,
    description TEXT NOT NULL CHECK (LENGTH(description) <= 1000),
    sphere_document tsvector GENERATED ALWAYS AS (
        to_tsvector('simple', description)
    ) STORED,
    is_nsfw BOOLEAN NOT NULL,
    is_banned BOOLEAN NOT NULL DEFAULT FALSE,
    icon_url TEXT,
    banner_url TEXT,
    num_members INT NOT NULL DEFAULT 0,
    creator_id BIGINT NOT NULL REFERENCES users (user_id),
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_search_sphere_name ON spheres(search_sphere_name);
CREATE INDEX idx_sphere_document ON spheres USING GIN (sphere_document);
CREATE INDEX idx_sphere_trigram ON spheres USING GIN (search_sphere_name gin_trgm_ops);

CREATE TABLE satellites (
    satellite_id BIGSERIAL PRIMARY KEY,
    satellite_name TEXT NOT NULL CHECK (LENGTH(satellite_name) <= 50),
    sphere_id BIGINT NOT NULL REFERENCES spheres (sphere_id),
    body TEXT NOT NULL CHECK (markdown_body IS NOT NULL OR LENGTH(body) <= 20000),
    markdown_body TEXT CHECK (LENGTH(markdown_body) <= 20000),
    is_nsfw BOOLEAN NOT NULL,
    is_spoiler BOOLEAN NOT NULL,
    num_posts INT NOT NULL DEFAULT 0,
    creator_id BIGINT NOT NULL REFERENCES users (user_id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disable_timestamp TIMESTAMPTZ,
    CONSTRAINT unique_satellite_name UNIQUE (satellite_name, sphere_id),
    CONSTRAINT unique_sphere UNIQUE (sphere_id, satellite_id)
);

-- index to guarantee unique satellite names per forum for active satellites
CREATE UNIQUE INDEX idx_unique_satellite ON satellites (sphere_id, satellite_name)
    WHERE satellites.disable_timestamp IS NULL;

CREATE TABLE user_sphere_roles (
    role_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users (user_id),
    sphere_id BIGINT NOT NULL REFERENCES spheres (sphere_id),
    permission_level TEXT NOT NULL CHECK (permission_level IN ('None', 'Moderate', 'Ban', 'Manage', 'Lead')),
    grantor_id BIGINT NOT NULL REFERENCES users (user_id),
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_timestamp TIMESTAMPTZ
);

-- index to guarantee there is only one role per user and sphere
CREATE UNIQUE INDEX idx_unique_sphere_role ON user_sphere_roles (sphere_id, user_id)
    WHERE user_sphere_roles.delete_timestamp IS NULL;
-- index to guarantee maximum 1 leader per sphere
CREATE UNIQUE INDEX idx_unique_sphere_leader ON user_sphere_roles (sphere_id, permission_level)
    WHERE user_sphere_roles.permission_level = 'Lead' AND user_sphere_roles.delete_timestamp IS NULL;

CREATE TABLE rules (
    rule_id BIGSERIAL PRIMARY KEY,
    rule_key BIGSERIAL, -- business id to track rule across updates
    sphere_id BIGINT REFERENCES spheres (sphere_id),
    priority SMALLINT NOT NULL,
    title TEXT NOT NULL CHECK (
        LENGTH(title) <= 250 AND (
            sphere_id IS NOT NULL OR
            title IN ('BeRespectful', 'RespectRules', 'NoIllegalContent', 'PlatformIntegrity')
        )
    ),
    description TEXT NOT NULL CHECK (markdown_description IS NOT NULL OR LENGTH(description) <= 500),
    markdown_description TEXT CHECK (LENGTH(description) <= 500),
    user_id BIGINT NOT NULL REFERENCES users (user_id),
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_timestamp TIMESTAMPTZ
);

-- index to guarantee numbering of rules is unique
CREATE UNIQUE INDEX idx_unique_rule ON rules (sphere_id, priority)
    WHERE rules.delete_timestamp IS NULL;
-- index to guarantee there is only one active rule for a given rule_key
CREATE UNIQUE INDEX idx_unique_rule_key ON rules (rule_key)
    WHERE rules.delete_timestamp IS NULL;

CREATE TABLE sphere_categories (
    category_id BIGSERIAL PRIMARY KEY,
    sphere_id BIGINT NOT NULL REFERENCES spheres (sphere_id),
    category_name TEXT NOT NULL CHECK (LENGTH(category_name) <= 50),
    category_color SMALLINT NOT NULL,
    description TEXT NOT NULL CHECK (LENGTH(description) <= 500),
    is_active BOOLEAN NOT NULL,
    creator_id BIGINT NOT NULL REFERENCES users (user_id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_timestamp TIMESTAMPTZ,
    CONSTRAINT sphere_category UNIQUE (category_id, sphere_id),
    CONSTRAINT unique_category UNIQUE (sphere_id, category_name)
);

CREATE INDEX idx_category_order ON sphere_categories (sphere_id, is_active, category_name);

CREATE TABLE sphere_subscriptions (
   subscription_id BIGSERIAL PRIMARY KEY,
   user_id BIGINT NOT NULL REFERENCES users (user_id),
   sphere_id BIGINT NOT NULL REFERENCES spheres (sphere_id),
   timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
   CONSTRAINT unique_subscription UNIQUE (user_id, sphere_id)
);

CREATE TABLE posts (
    post_id BIGSERIAL PRIMARY KEY,
    title TEXT NOT NULL CHECK (LENGTH(title) <= 250),
    body TEXT NOT NULL CHECK (markdown_body IS NOT NULL OR LENGTH(body) <= 20000),
    markdown_body TEXT CHECK (LENGTH(markdown_body) <= 20000),
    post_document tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('simple', title), 'A') ||
        setweight(to_tsvector('simple', coalesce(markdown_body, body)), 'B')
    ) STORED,
    link_type SMALLINT NOT NULL CHECK (link_type IN (-1, 0, 1, 2, 3)),
    link_url TEXT CHECK (LENGTH(link_url) <= 500),
    link_embed TEXT,
    link_thumbnail_url TEXT,
    is_nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    is_spoiler BOOLEAN NOT NULL DEFAULT FALSE,
    category_id BIGINT,
    is_edited BOOLEAN NOT NULL DEFAULT FALSE,
    sphere_id BIGINT NOT NULL REFERENCES spheres (sphere_id),
    satellite_id BIGINT,
    creator_id BIGINT NOT NULL REFERENCES users (user_id),
    is_creator_moderator BOOLEAN NOT NULL,
    moderator_message TEXT CHECK (LENGTH(moderator_message) <= 500),
    infringed_rule_id BIGINT REFERENCES rules (rule_id),
    moderator_id BIGINT REFERENCES users (user_id),
    num_comments INT NOT NULL DEFAULT 0,
    is_pinned BOOLEAN NOT NULL,
    score INT NOT NULL DEFAULT 0,
    score_minus INT NOT NULL DEFAULT 0,
    recommended_score REAL NOT NULL GENERATED ALWAYS AS (
        LOG10(score_mapping(score)) - 3 * EXTRACT(EPOCH FROM (scoring_timestamp - create_timestamp))/(3600 * 24 * 2)
    ) STORED,
    trending_score REAL NOT NULL GENERATED ALWAYS AS (
        LOG10(score_mapping(score)) - EXTRACT(EPOCH FROM (scoring_timestamp - create_timestamp))/(3600 * 2)
    ) STORED,
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    edit_timestamp TIMESTAMPTZ,
    scoring_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_timestamp TIMESTAMPTZ,
    CONSTRAINT valid_satellite FOREIGN KEY (sphere_id, satellite_id) REFERENCES satellites (sphere_id, satellite_id) MATCH SIMPLE,
    CONSTRAINT valid_sphere_category FOREIGN KEY (category_id, sphere_id) REFERENCES sphere_categories (category_id, sphere_id) MATCH SIMPLE
);

-- Global score indexes for 4 score types
CREATE INDEX idx_posts_recommended ON posts (recommended_score DESC)
    WHERE moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL AND satellite_id IS NULL;
CREATE INDEX idx_posts_trending ON posts (trending_score DESC)
    WHERE moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL AND satellite_id IS NULL;
CREATE INDEX idx_posts_best ON posts (score DESC)
    WHERE moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL AND satellite_id IS NULL;
CREATE INDEX idx_posts_recent  ON posts (create_timestamp DESC)
    WHERE moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL AND satellite_id IS NULL;

-- Sphere indexes
CREATE INDEX idx_posts_sphere_recommended
    ON posts (sphere_id, recommended_score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL;

CREATE INDEX idx_posts_sphere_trending
    ON posts (sphere_id, trending_score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL;

CREATE INDEX idx_posts_sphere_best
    ON posts (sphere_id, score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL;

CREATE INDEX idx_posts_sphere_recent
    ON posts (sphere_id, create_timestamp DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL;

-- Satellite only indexes, very small so ok to have all four variants
CREATE INDEX idx_posts_satellite_recommended
    ON posts (satellite_id, recommended_score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NOT NULL;

CREATE INDEX idx_posts_satellite_trending
    ON posts (satellite_id, trending_score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NOT NULL;

CREATE INDEX idx_posts_satellite_best
    ON posts (satellite_id, score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NOT NULL;

CREATE INDEX idx_posts_satellite_recent
    ON posts (satellite_id, create_timestamp DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NOT NULL;

-- Pinned-only indexes for fast lookup
CREATE INDEX idx_posts_pinned_sphere
    ON posts (sphere_id, recommended_score DESC)
    INCLUDE (trending_score, score, create_timestamp)
    WHERE is_pinned = TRUE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NULL;

CREATE INDEX idx_posts_pinned_satellite
    ON posts (satellite_id, recommended_score DESC)
    INCLUDE (trending_score, score, create_timestamp)
    WHERE is_pinned = TRUE AND moderator_id IS NULL AND delete_timestamp IS NULL AND satellite_id IS NOT NULL;

CREATE INDEX idx_post_creator
    ON posts (creator_id, recommended_score DESC)
    INCLUDE (trending_score, score, create_timestamp)
    WHERE moderator_id IS NULL AND delete_timestamp IS NULL;

-- Index on post document for searches
CREATE INDEX idx_post_document ON posts USING GIN (post_document);

CREATE TABLE comments (
    comment_id BIGSERIAL PRIMARY KEY,
    body TEXT NOT NULL CHECK (markdown_body IS NOT NULL OR LENGTH(body) <= 20000),
    markdown_body TEXT CHECK (LENGTH(markdown_body) <= 20000),
    comment_document TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('simple', coalesce(markdown_body, body))
    ) STORED,
    is_edited BOOLEAN NOT NULL DEFAULT FALSE,
    moderator_message TEXT CHECK (LENGTH(moderator_message) <= 500),
    infringed_rule_id BIGINT REFERENCES rules (rule_id),
    parent_id BIGINT REFERENCES comments (comment_id),
    post_id BIGINT NOT NULL REFERENCES posts (post_id),
    creator_id BIGINT NOT NULL REFERENCES users (user_id),
    is_creator_moderator BOOLEAN NOT NULL,
    moderator_id BIGINT REFERENCES users (user_id),
    is_pinned BOOLEAN NOT NULL,
    score INT NOT NULL DEFAULT 0,
    score_minus INT NOT NULL DEFAULT 0,
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    edit_timestamp TIMESTAMPTZ,
    delete_timestamp TIMESTAMPTZ
);

-- Post indexes
CREATE INDEX idx_comments_post_best
    ON comments (post_id, parent_id, is_pinned, score DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL;

CREATE INDEX idx_comments_post_recent
    ON comments (post_id, parent_id, is_pinned, create_timestamp DESC)
    WHERE is_pinned = FALSE AND moderator_id IS NULL AND delete_timestamp IS NULL;

CREATE INDEX idx_comment_creator
    ON comments (creator_id, score DESC)
    INCLUDE (create_timestamp)
    WHERE moderator_id IS NULL AND delete_timestamp IS NULL;

CREATE INDEX idx_comment_document ON comments USING GIN (comment_document);

CREATE TABLE votes (
    vote_id BIGSERIAL PRIMARY KEY,
    post_id BIGINT NOT NULL REFERENCES posts (post_id),
    comment_id BIGINT REFERENCES comments (comment_id),
    user_id BIGINT NOT NULL REFERENCES users (user_id),
    value SMALLINT NOT NULL CHECK (value IN (-1, 1)),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_vote UNIQUE NULLS NOT DISTINCT (post_id, comment_id, user_id)
);

CREATE INDEX idx_vote_context ON votes (user_id, post_id, comment_id);

CREATE TABLE user_bans (
    ban_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users (user_id),
    sphere_id BIGINT REFERENCES spheres (sphere_id),
    post_id BIGINT NOT NULL,
    comment_id BIGINT,
    infringed_rule_id BIGINT NOT NULL REFERENCES rules (rule_id),
    moderator_id BIGINT NOT NULL REFERENCES users (user_id),
    until_timestamp TIMESTAMPTZ,
    create_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delete_timestamp TIMESTAMPTZ
);

CREATE INDEX idx_user_ban_sphere ON user_bans (sphere_id, user_id, delete_timestamp)
    WHERE user_bans.delete_timestamp IS NULL;

-- add functional user
INSERT INTO users (oidc_id, username, email)
VALUES ('', 'sphare-function-user', '');

-- add base rules
INSERT INTO rules (sphere_id, priority, title, description, markdown_description, user_id)
VALUES
(
null, 0,'BeRespectful', '',
 $$Treat all members of the community with respect. Healthy debate and differing opinions are welcome, but personal attacks, insults, or hostile behavior will not be tolerated.

Always aim to contribute constructively and make Sphare a space where everyone feels safe to participate.$$,
1
),
(
null, 1,'RespectRules', '',
$$Post in the appropriate communities and follow their rules. Make sure to use accurate titles, tags, and categories to help others understand the topic of your post. Stay on-topic and contribute in good faith on topics where you have a genuine interest - this helps keep communities organized, relevant, and welcoming for everyone.

Furthermore, mature content that is not suitable for children (sexually explicit, graphic, violent or offensive) and spoilers must be labelled as NSFW and Spoilers respectively. You can find more details in our [Content Policy](/content_policy).$$,
1
),
(
null, 2,'NoIllegalContent', '',
$$Any illegal content, content advocating or soliciting illegal acts or transactions and malicious content that aims to cause harm or negatively impact other users is strictly prohibited. More detail can be found in our [Content Policy](/content_policy).

Violating this rule will lead to immediate removal of content and a permanent ban. Depending on the infraction, it can also be reported to authorities.$$,
1
),
(
null, 3,'PlatformIntegrity', '',
 $$Do not engage in any activity that harms, disrupts, or undermines the platform or its users. This includes (but is not limited to) inserting malicious code, exploiting bugs, evading bans, bypassing security measures, or helping others do so.

Do not attempt to deceive users, manipulating votes or reputation systems, or make the site harder to use through spam, flooding, or disruptive behavior. Keep the platform safe, fair, and accessible for everyone.$$,
1);