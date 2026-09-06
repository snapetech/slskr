FROM node:22-bookworm AS web-builder

ARG VERSION=dev
WORKDIR /src

COPY web/.npmrc web/package*.json web/
RUN npm --prefix web ci
COPY web web
RUN npm --prefix web run build

FROM rust:1.94-bookworm AS builder

ARG VERSION=dev
WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .
COPY --from=web-builder /src/web/build web/build
RUN SLSKR_RELEASE_VERSION="${VERSION}" cargo build --release -p slskr --locked

FROM debian:bookworm-slim

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILD_DATE=unknown

LABEL org.opencontainers.image.title="slskr" \
      org.opencontainers.image.description="Rust Soulseek daemon with bundled Web UI" \
      org.opencontainers.image.source="https://github.com/snapetech/slskr" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.licenses="AGPL-3.0-only"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/* \
    && if ! getent group slskr >/dev/null; then \
         existing_group="$(getent group 1000 | cut -d: -f1 || true)"; \
         if [ -n "$existing_group" ]; then groupmod --new-name slskr "$existing_group"; else groupadd --gid 1000 slskr; fi; \
       fi \
    && if ! getent passwd slskr >/dev/null; then \
         existing_user="$(getent passwd 1000 | cut -d: -f1 || true)"; \
         if [ -n "$existing_user" ]; then usermod --login slskr --home /var/lib/slskr --shell /usr/sbin/nologin "$existing_user"; else useradd --system --uid 1000 --gid 1000 --home-dir /var/lib/slskr --create-home --shell /usr/sbin/nologin slskr; fi; \
       fi \
    && mkdir -p /usr/share/slskr/web /etc/slskr /var/lib/slskr \
    && chown -R slskr:slskr /var/lib/slskr

COPY --from=builder /src/target/release/slskr /usr/local/bin/slskr
COPY --from=builder /src/web/build /usr/share/slskr/web/build
COPY docs/slskr.config.example.toml /etc/slskr/config.toml.example

USER slskr
EXPOSE 5030 2234
ENV SLSKR_HTTP_BIND=0.0.0.0:5030 \
    SLSKR_STATE_DIR=/var/lib/slskr \
    SLSKR_WEB_BUILD_DIR=/usr/share/slskr/web/build

HEALTHCHECK --interval=60s --timeout=3s --start-period=60s --retries=3 \
    CMD wget -q -O - http://localhost:5030/health || exit 1

# Keep the daemon's shutdown handler in control of container termination.
STOPSIGNAL SIGTERM
ENTRYPOINT ["slskr"]
CMD ["serve"]
