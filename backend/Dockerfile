FROM rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --locked --release --bins

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 oppor && useradd --uid 10001 --gid 10001 --no-create-home oppor
COPY --from=build /app/target/release/oppor-api /usr/local/bin/oppor-api
COPY --from=build /app/target/release/oppor-worker /usr/local/bin/oppor-worker
USER 10001:10001
EXPOSE 8080
ENV HTTP_BIND=0.0.0.0:8080 APP_ENV=production
ENTRYPOINT ["oppor-api"]
