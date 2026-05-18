FROM node:22-bookworm AS web-builder

ARG VERSION=dev
WORKDIR /src

COPY web/package*.json web/
RUN npm --prefix web ci
COPY web web
RUN npm --prefix web run build

FROM rust:1.93-bookworm AS builder

ARG VERSION=dev
WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .
COPY --from=web-builder /src/web/build web/build
RUN SLSKR_RELEASE_VERSION="${VERSION}" cargo build --release -p slskr

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
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home-dir /var/lib/slskr --create-home --shell /usr/sbin/nologin slskr \
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

ENTRYPOINT ["slskr"]
CMD ["serve"]
