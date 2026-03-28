-- 001-schema.sql
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

CREATE TABLE IF NOT EXISTS admin_user (
    id BIGSERIAL PRIMARY KEY,
    uuid uuid NOT NULL DEFAULT UUIDV7(),
    date_created timestamptz NOT NULL DEFAULT NOW(),
    date_updated timestamptz NOT NULL DEFAULT NOW(),
    row_version INTEGER NOT NULL DEFAULT 1,

    username TEXT NOT NULL UNIQUE,
    password_salt TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    password_iterations INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS db_meta (
    id BIGSERIAL PRIMARY KEY,
    uuid uuid NOT NULL DEFAULT UUIDV7(),
    date_created timestamptz NOT NULL DEFAULT NOW(),
    date_updated timestamptz NOT NULL DEFAULT NOW(),
    row_version INTEGER NOT NULL DEFAULT 1,

    schema_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS greeting (
    id BIGSERIAL PRIMARY KEY,
    uuid uuid NOT NULL DEFAULT UUIDV7(),
    date_created timestamptz NOT NULL DEFAULT NOW(),
    date_updated timestamptz NOT NULL DEFAULT NOW(),
    row_version INTEGER NOT NULL DEFAULT 1,

    message TEXT NOT NULL
);
