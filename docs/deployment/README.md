# Deployment

Velora production runs on a single Docker host: the Panel, the documentation site and
the operations dashboard, with nightly offsite backups. See
[targets](../DEPLOYMENT_TARGETS.md), the [deployment guide](DEPLOYMENT_GUIDE.md) and the
[single-host setup](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/vps/README.md).

The [operations dashboard](https://github.com/VeloraMCDev/velora-launcher/blob/main/panel/operations/README.md)
shows health and activity, takes backups, deploys new builds and approves launcher
releases. The [Cloudflare control plane](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/control-plane/README.md)
and [native agent](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/agent/README.md)
remain an optional multi-host design.

Keep live hostnames, addresses, credentials and backups in ignored operator
configuration. Public examples are synthetic.
