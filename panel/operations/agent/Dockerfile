FROM node:24-alpine
ARG AGENT_REVISION=development
LABEL org.opencontainers.image.title="Velora Monitoring Agent" \
      org.opencontainers.image.source="https://github.com/VeloraMCDev/velora-launcher" \
      org.opencontainers.image.revision=$AGENT_REVISION
WORKDIR /app
COPY --chown=node:node monitor.mjs ./monitor.mjs
RUN mkdir /config && chown node:node /config
USER node
ENTRYPOINT ["node", "/app/monitor.mjs"]
CMD ["/config/velora-monitor-config.json"]
