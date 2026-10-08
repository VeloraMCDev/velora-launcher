# Hermes Development and Beta host

Operator-confirmed on 2026-10-07: Hermes is a Linux Mint machine with Docker
already running. Use a native systemd deployment agent. SSH may be used for
installation/recovery; regular jobs use authenticated outbound HTTPS to the
Cloudflare control plane. The Windows migration workspace is a separate machine.

SSH key authentication and the server fingerprint are verified. The read-only
preflight checked architecture, systemd, Docker and Compose. Live version,
capacity and connection details belong in ignored operator evidence. These
checks do not certify a dedicated agent service account or its disk budget.
Sudo requires a local password; unrestricted passwordless sudo was not enabled.

Run this read-only check on Hermes as the intended service user:

```sh
bash scripts/hermes-preflight.sh
```

It checks Linux, systemd, the Docker daemon, Compose plugin and the initial
probe's `linux/amd64` platform. It prints no environment, Docker configuration,
credentials or workload inventory. Each Docker call has a 15-second timeout.
It creates no containers or services. It passed on Hermes on 2026-10-07.
An ARM host needs a verified ARM artifact before the probe can be deployed.

Before enrollment, verify actual storage/architecture and agree separate
Development/Beta Compose projects, persistent storage, credentials, network
ports and capabilities. Production is not an implicit capability of Hermes.
Docker access is effectively host administration: grant it only to the native
agent service account, keep agent keys owner-readable, and expose neither the
Docker daemon nor a generic shell over the network. Do not put broad Cloudflare
or GitHub credentials on Hermes. Enrollment and service installer remain absent.

Connection details and the pinned known-hosts file live under the ignored
`.runtime-checks/` operator directory. Windows SSH-agent custody keeps the
passphrase out of automation commands. No secret values belong in this document
or chat. Next: candidate registration, followed by dedicated agent enrollment.

## First-time SSH setup

The following first-time instructions were used during setup. On Hermes, open Terminal:

```sh
sudo apt update
sudo apt install openssh-server
sudo systemctl enable --now ssh
systemctl is-active ssh
whoami
hostname -I
sudo ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub
```

Enter the sudo password only in the local terminal. Record the local username,
LAN address and SHA256 host fingerprint in the ignored operator configuration.
Verify that fingerprint before trusting the first Windows SSH connection.
Keep both machines on the same local network; router port forwarding is unnecessary.
If a firewall is already active, allow TCP 22 from the Windows machine's LAN
address only. Do not disable the firewall or open SSH to the entire Internet.

Next, create a dedicated, passphrase-protected Ed25519 SSH key on the operator's
Windows machine, install only its public key in the intended user's
`~/.ssh/authorized_keys`, and test key authentication before changing password
authentication settings. Preserve existing SSH configuration and keys. An agent
enrollment key is separate from this administrative SSH key. Installer and agent
privileges still need review; a successful SSH login does not authorize Production.

Reference: [Ubuntu OpenSSH server setup](https://ubuntu.com/server/docs/openssh-server/).
