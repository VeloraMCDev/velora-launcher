FROM node:24-alpine
WORKDIR /app
COPY --chown=node:node monitor.mjs ./monitor.mjs
RUN mkdir /config && chown node:node /config
USER node
ENTRYPOINT ["node", "/app/monitor.mjs"]
CMD ["/config/velora-monitor-config.json"]
