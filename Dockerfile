# Dockerfile

FROM rust:1.94-slim AS builder

WORKDIR /app

ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY src ./src
COPY static ./static
COPY templates ./templates
COPY locales ./locales

RUN cargo build --locked --release


FROM debian:trixie-slim AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/caritas-love /usr/local/bin/caritas-love
COPY static ./static
COPY templates ./templates
COPY locales ./locales

ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000

EXPOSE 3000

CMD ["caritas-love"]
