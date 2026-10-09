# Deployment

Velora supports Cloudflare, personally hosted machines and VPS deployments through
a shared deployment authority and independent service/artifact contracts.

Start with [targets and domains](../DEPLOYMENT_TARGETS.md) and the
[deployment guide](DEPLOYMENT_GUIDE.md). The [Cloudflare control plane](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/control-plane/README.md)
owns coordination; the [native agent](https://github.com/VeloraMCDev/velora-launcher/blob/main/infra/agent/README.md) runs outbound
jobs on native hosts. The [Deployment Panel](https://github.com/VeloraMCDev/velora-launcher/blob/main/panel/deployment-ui/README.md)
provides the protected operator interface.

Keep live resource IDs, host addresses, enrollment output, credentials and backups
in ignored operator state. Public examples are synthetic. Source CI is enabled;
deployment promotion remains manual until source policies and production gates
have been reviewed for this repository.
