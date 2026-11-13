FROM rust:1.97.1-bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --locked --release --bins

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 oppor && useradd --uid 10001 --gid 10001 --no-create-home oppor
COPY --from=build /app/target/release/oppor-api /usr/local/bin/oppor-api
COPY --from=build /app/target/release/oppor-worker /usr/local/bin/oppor-worker
USER 10001:10001
EXPOSE 8080
ENV HTTP_BIND=0.0.0.0:8080 APP_ENV=production
ENTRYPOINT ["oppor-api"]
