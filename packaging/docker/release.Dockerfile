FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS release

ARG TARGETARCH
ARG VERSION=dev

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tar \
    && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    case "$TARGETARCH" in \
      amd64) rust_target="x86_64-unknown-linux-gnu" ;; \
      arm64) rust_target="aarch64-unknown-linux-gnu" ;; \
      *) echo "unsupported Docker target architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    test "$VERSION" != "dev"; \
    archive="slskr-${VERSION}-${rust_target}.tar.gz"; \
    url="https://github.com/snapetech/slskr/releases/download/release-${VERSION}/${archive}"; \
    curl -fsSL "$url" -o /tmp/slskr.tar.gz; \
    mkdir -p /out; \
    tar -xzf /tmp/slskr.tar.gz -C /out --strip-components=1; \
    test -x /out/slskr; \
    test -f /out/web/build/index.html; \
    test -f /out/docs/slskr.config.example.toml

FROM ubuntu:24.04

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

COPY --from=release /out/slskr /usr/local/bin/slskr
COPY --from=release /out/web/build /usr/share/slskr/web/build
COPY --from=release /out/docs/slskr.config.example.toml /etc/slskr/config.toml.example

USER slskr
EXPOSE 5030 2234
ENV SLSKR_HTTP_BIND=0.0.0.0:5030 \
    SLSKR_STATE_DIR=/var/lib/slskr \
    SLSKR_WEB_BUILD_DIR=/usr/share/slskr/web/build

ENTRYPOINT ["slskr"]
CMD ["serve"]
