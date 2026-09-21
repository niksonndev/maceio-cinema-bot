FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/maceio-cine-whatsapp /usr/local/bin/maceio-cine-whatsapp
ENV PORT=10000
ENV SESSION_PATH=/tmp/whatsapp.db
EXPOSE 10000
CMD ["maceio-cine-whatsapp"]
