# Velora Operations dashboard. Build from the repository root:
#   docker build -f panel/operations/Dockerfile -t velora-operations panel/operations
FROM node:24-alpine AS web
WORKDIR /src
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web ./web
COPY server ./server
RUN npm run check && npm run check:runes && npm run build && npm test

FROM node:24-alpine
ARG OPS_VERSION=development
ARG OPS_REVISION=unknown
LABEL org.opencontainers.image.title="Velora Operations" \
      org.opencontainers.image.description="Health, deployment and release control for a Velora stack" \
      org.opencontainers.image.source="https://github.com/VeloraMCDev/velora-launcher" \
      org.opencontainers.image.revision=$OPS_REVISION \
      org.opencontainers.image.version=$OPS_VERSION
ENV NODE_ENV=production OPS_VERSION=$OPS_VERSION PORT=8080 OPS_DATA_DIR=/data
WORKDIR /app
COPY --from=web /src/server ./server
COPY --from=web /src/web/dist ./web/dist
COPY package.json ./
RUN rm -f server/*.test.mjs && mkdir -p /data && chown node:node /data
USER node
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s CMD wget -qO- http://127.0.0.1:8080/healthz >/dev/null || exit 1
CMD ["node", "server/index.mjs"]
