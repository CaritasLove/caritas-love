-- 20260324220000_base_schema.sql
-- Copyright 2026 Patrick Meade.
--
-- This program is free software: you can redistribute it and/or modify
-- it under the terms of the GNU Affero General Public License as published by
-- the Free Software Foundation, either version 3 of the License, or
-- (at your option) any later version.
--
-- This program is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU Affero General Public License for more details.
--
-- You should have received a copy of the GNU Affero General Public License
-- along with this program.  If not, see <https://www.gnu.org/licenses/>.

CREATE TABLE IF NOT EXISTS greeting (
    id BIGSERIAL PRIMARY KEY,
    message TEXT NOT NULL
);

INSERT INTO greeting (id, message)
VALUES (1, 'Hello, world!')
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS admin_user (
    id BIGSERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    force_password_change BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS admin_session (
    id BIGSERIAL PRIMARY KEY,
    admin_user_id BIGINT NOT NULL REFERENCES admin_user(id) ON DELETE CASCADE,
    session_token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL,
    client_label TEXT NULL
);

CREATE INDEX IF NOT EXISTS admin_session_admin_user_id_idx
    ON admin_session (admin_user_id);

CREATE INDEX IF NOT EXISTS admin_session_active_lookup_idx
    ON admin_session (session_token_hash, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS admin_audit_event (
    id BIGSERIAL PRIMARY KEY,
    actor_admin_user_id BIGINT NULL REFERENCES admin_user(id) ON DELETE SET NULL,
    target_admin_user_id BIGINT NULL REFERENCES admin_user(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    detail TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS admin_audit_event_event_type_idx
    ON admin_audit_event (event_type, created_at DESC);
