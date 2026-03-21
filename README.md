# caritas-love
Administration and Operations Support Software for Community Organizations

## Environment Variables

The application loads environment variables from `.env` on startup (`dotenvy`).

| Variable | Required | Default | Notes |
| --- | --- | --- | --- |
| `DATABASE_URL` | Yes | none | PostgreSQL connection string used by the app at runtime. |
| `APP_HOST` | No | `127.0.0.1` | Host/IP address the HTTP server binds to. |
| `APP_PORT` | No | `3000` | Port the HTTP server binds to. |
| `APP_ENV` | No | `production` | Runtime mode. Accepted values (case-insensitive): `production`, `prod`, `development`, `dev`. Unknown values log a warning and fall back to `production`. |
| `SQLX_OFFLINE` | No | unset | Primarily a development/build setting for SQLx query checking with the local `.sqlx` cache. |

Example `.env`:

```dotenv
APP_ENV=development
APP_HOST=127.0.0.1
APP_PORT=3000
DATABASE_URL=postgres://caritas:caritas_dev_password@localhost:5432/caritas
SQLX_OFFLINE=true
```

## Development

Install the command line tools for sqlx:

    cargo install sqlx-cli

Update the cached data for offline development:

    cargo sqlx prepare
