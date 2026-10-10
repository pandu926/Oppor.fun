# Build the binaries first with the pinned Rust toolchain and locked dependencies.
FROM debian:trixie-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 oppor && useradd --uid 10001 --gid 10001 --no-create-home oppor
COPY backend/target/release/oppor-api /usr/local/bin/oppor-api
COPY backend/target/release/oppor-worker /usr/local/bin/oppor-worker
USER 10001:10001
ENV HTTP_BIND=0.0.0.0:8080 APP_ENV=production
EXPOSE 8080
ENTRYPOINT ["oppor-api"]
