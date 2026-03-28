-- 002-data.sql
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

INSERT INTO admin_user (
    username,
    password_salt,
    password_hash,
    password_iterations
)
VALUES (
    'admin',
    'zfRi1EPSWttRorSZSZSg3Q==',
    '1GJi0qUSTfD1epcMoy0u6IFNXcH1cerQzAtJujHBY18=',
    600000
);

INSERT INTO db_meta (
    schema_version
)
VALUES (
    1
);

INSERT INTO greeting (
    message
)
VALUES (
    'Hello, world!'
);
