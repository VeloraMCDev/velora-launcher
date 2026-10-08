# syntax=docker/dockerfile:1.7
#
# Velora admin panel — a single static binary + the web UI on `scratch`.
# Multi-arch (amd64/arm64) is cross-compiled with cargo-zigbuild on the
# build machine's native arch, so no slow QEMU emulation is needed.

# ---- 1. Web UI ---------------------------------------------------------------
FROM --platform=$BUILDPLATFORM node:22-alpine AS web
WORKDIR /src/panel/web
COPY panel/web/package.json panel/web/package-lock.json ./
RUN npm ci --no-audit --no-fund
# The map viewer is shared with the launcher (imported as @scopenet/map).
COPY shared/ /src/shared/
COPY packages/platform-ui/ /src/packages/platform-ui/
COPY packages/platform-map/ /src/packages/platform-map/
COPY packages/panel-ui/ /src/packages/panel-ui/
COPY packages/platform-skin/ /src/packages/platform-skin/
COPY panel/web/ ./
RUN npm run build

# ---- 1b. Icon library ---------------------------------------------------------
FROM --platform=$BUILDPLATFORM node:22-alpine AS icons
WORKDIR /src/panel/icons
COPY panel/icons/package.json panel/icons/package-lock.json panel/icons/build.mjs ./
RUN npm ci --no-audit --no-fund && node build.mjs

# ---- 2. Server binary --------------------------------------------------------
FROM --platform=$BUILDPLATFORM rust:1-slim-bookworm AS build
ARG TARGETARCH
# zig (via the cargo-zigbuild wheel) is the cross C toolchain for musl targets.
RUN apt-get update && apt-get install -y --no-install-recommends python3-pip \
 && rm -rf /var/lib/apt/lists/* \
 && pip install --no-cache-dir --break-system-packages cargo-zigbuild ziglang
RUN case "${TARGETARCH:-$(uname -m)}" in \
      amd64|x86_64) echo x86_64-unknown-linux-musl ;; \
      arm64|aarch64) echo aarch64-unknown-linux-musl ;; \
      *) echo "unsupported arch ${TARGETARCH:-$(uname -m)}" >&2; exit 1 ;; \
    esac > /rust-target \
 && rustup target add "$(cat /rust-target)"
WORKDIR /src
COPY . .
# Stamped into the binary (GET /health) and the image labels; changing it rebuilds only the panel crate.
ARG VELORA_BUILD_REVISION=unknown
ENV VELORA_BUILD_REVISION=$VELORA_BUILD_REVISION
# Release identity is supplied through build arguments; dependency resolution stays locked.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry,sharing=locked \
    --mount=type=cache,target=/src/target,id=scopenet-target-$TARGETARCH \
    cargo zigbuild --locked --release -p scopenet-panel --target "$(cat /rust-target)" \
 && cp "target/$(cat /rust-target)/release/scopenet-panel" /scopenet-panel
# Data dir owned by the non-root runtime user.
RUN mkdir -p /out/data && chown 65532:65532 /out/data

# SQLite needs temporary storage for migrations and attached instance databases.
# Prepare it separately so the prebuilt target does not compile the application.
FROM --platform=$BUILDPLATFORM busybox:1.37 AS runtime-dirs
RUN mkdir -p /out/tmp && chmod 1777 /out/tmp

# ---- 3a. Runtime from a binary built outside Docker ---------------------------
# Used by the release workflow, which compiles the panel natively (with a warm Cargo cache) and only packages it here:
#   docker build --target prebuilt <context holding scopenet-panel, web/ and data/.keep>
# Build this file without --target (docker compose does) to compile everything inside Docker instead.
FROM scratch AS prebuilt
LABEL org.opencontainers.image.title="Velora Panel" \
      org.opencontainers.image.description="Admin panel for the Velora Minecraft launcher" \
      org.opencontainers.image.source="https://github.com/VeloraMCDev/experiences"
COPY scopenet-panel /scopenet-panel
COPY web /web
COPY icons /icons
COPY --chown=65532:65532 data /data
COPY --from=runtime-dirs /out/ /
ENV SCOPENET_BIND=0.0.0.0:8080 \
    TMPDIR=/tmp \
    SCOPENET_DATA_DIR=/data \
    SCOPENET_WEB_DIR=/web \
    SCOPENET_ICONS_DIR=/icons \
    RUST_LOG=info
USER 65532:65532
VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/scopenet-panel", "healthcheck"]
ENTRYPOINT ["/scopenet-panel"]

# ---- 3b. Runtime --------------------------------------------------------------
FROM scratch
ARG VELORA_BUILD_REVISION=unknown
ARG VELORA_BUILD_VERSION=0.0.0
ARG VELORA_BUILD_CREATED=unknown
LABEL org.opencontainers.image.title="Velora Panel" \
      org.opencontainers.image.description="Admin panel for the Velora Minecraft launcher" \
      org.opencontainers.image.source="https://github.com/VeloraMCDev/experiences" \
      org.opencontainers.image.revision=$VELORA_BUILD_REVISION \
      org.opencontainers.image.version=$VELORA_BUILD_VERSION \
      org.opencontainers.image.created=$VELORA_BUILD_CREATED
COPY --from=build /scopenet-panel /scopenet-panel
COPY --from=web /src/panel/web/dist /web
COPY --from=icons /src/panel/icons/dist /icons
COPY --from=build --chown=65532:65532 /out/data /data
COPY --from=runtime-dirs /out/ /
ENV SCOPENET_BIND=0.0.0.0:8080 \
    TMPDIR=/tmp \
    SCOPENET_DATA_DIR=/data \
    SCOPENET_WEB_DIR=/web \
    SCOPENET_ICONS_DIR=/icons \
    RUST_LOG=info
USER 65532:65532
VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/scopenet-panel", "healthcheck"]
ENTRYPOINT ["/scopenet-panel"]
