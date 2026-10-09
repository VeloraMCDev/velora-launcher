# One repository, independent boundaries

VeloraMCDev/velora-launcher contains the complete product. The repository name
does not limit it to the desktop launcher. Each maintained segment has its own
README, build inputs and deployment lifecycle.

Reusable authentication, SDK and Panel libraries cannot depend on gameplay or
the complete Panel application. The complete application currently composes
gameplay directly; a formal extension interface remains future work.

The root Rust workspace owns the applications and reusable crates. Infra retains
an excluded nested workspace and lockfile. Frontends keep their own npm lockfiles;
Java integration builds remain separated by loader and toolchain.

Start with [segment boundaries](docs/BOUNDARIES.md), [local development](docs/APPLICATION.md),
[validation](docs/VALIDATION.md) and [deployment targets](docs/DEPLOYMENT_TARGETS.md).
Historical duplicate source and project migration reports have been removed from
the maintained tree. Component API contracts and attribution remain under
docs/components and package provenance files. Runtime SQL schema migrations stay
in place because they are required to create and upgrade service databases.

Installed launcher IDs, signing/credential identities, Minecraft namespaces,
database paths and persistent volume names remain compatible. Source organization
does not silently promote releases, change live state or provision paid capacity.
