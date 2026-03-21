-- 001-schema.sql

CREATE TABLE IF NOT EXISTS greeting (
    id BIGSERIAL PRIMARY KEY,
    message TEXT NOT NULL
);

INSERT INTO greeting (message)
VALUES ('Hello, world!')
ON CONFLICT DO NOTHING;
