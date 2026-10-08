FROM node:24-alpine@sha256:ebfe2f90462722a7a4de65e91990e97fe0d401c70e0e762c5b53302f905ec1c1
WORKDIR /app
ARG BUILD_REVISION
ARG BUILD_RUN_ID
ARG BUILD_VERSION=0.1.0
ARG BUILD_CREATED
LABEL org.opencontainers.image.source="https://github.com/VeloraMCDev/velora-launcher" \
      org.opencontainers.image.title="Velora Development Deployment Probe" \
      org.opencontainers.image.revision=$BUILD_REVISION \
      org.opencontainers.image.version=$BUILD_VERSION \
      org.opencontainers.image.created=$BUILD_CREATED
ENV BUILD_REVISION=$BUILD_REVISION BUILD_RUN_ID=$BUILD_RUN_ID BUILD_VERSION=$BUILD_VERSION
RUN node -e 'const fs=require("node:fs");const commit=process.env.BUILD_REVISION,build=process.env.BUILD_RUN_ID,version=process.env.BUILD_VERSION;if(!/^[a-f0-9]{40}$/.test(commit)||!/^[a-zA-Z0-9_.-]{1,128}$/.test(build)||!/^\d+\.\d+\.\d+$/.test(version))throw Error("Invalid build identity");fs.writeFileSync("build-info.json",JSON.stringify({commit,build,version}));'
COPY server.mjs ./
USER node
EXPOSE 8080
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
  CMD node -e 'fetch("http://127.0.0.1:8080/health/ready").then(r=>{if(!r.ok)process.exit(1)}).catch(()=>process.exit(1))'
CMD ["node", "server.mjs"]
