# caritas-love
Administration and Operations Support Software for Community Organizations

## License
Copyright 2026 Patrick Meade

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.

## Environment Variables

The application loads environment variables from `.env` on startup (`dotenvy`).

| Variable | Required | Default | Notes |
| --- | --- | --- | --- |
| `APP_ENV` | No | `production` | Runtime mode. Accepted values (case-insensitive): `production`, `prod`, `development`, `dev`. Unknown values log a warning and fall back to `production`. |
| `APP_HOST` | No | `127.0.0.1` | Host/IP address the HTTP server binds to. |
| `APP_PORT` | No | `3000` | Port the HTTP server binds to. |
| `DATABASE_URL` | Yes | none | PostgreSQL connection string used by the app at runtime. |
| `RUST_LOG` | No | `info` | Logging configuration for the application; see [env_logger](https://docs.rs/env_logger/latest/env_logger/#enabling-logging). |
| `SQLX_OFFLINE` | No | unset | Primarily a development/build setting for SQLx query checking with the local `.sqlx` cache. |

Example `.env`:

```dotenv
APP_ENV=development
APP_HOST=127.0.0.1
APP_PORT=3000
DATABASE_URL=postgres://caritas:caritas_dev_password@localhost:5432/caritas
RUST_LOG=trace
SQLX_OFFLINE=true
```

## Development

Install the command line tools for sqlx:

    cargo install sqlx-cli

Update the cached data for offline development:

    cargo sqlx prepare
