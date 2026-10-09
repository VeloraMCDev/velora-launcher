# Velora documentation

Build, understand and operate the complete Velora product from one source tree.

| Start here | What it covers |
|---|---|
| [Application](APPLICATION.md) | Panel and desktop development, fresh local stores and acceptance |
| [Boundaries](BOUNDARIES.md) | Segment ownership and allowed dependencies |
| [Known limitations](KNOWN_ISSUES.md) | Runtime and deployment gaps found during review |
| [Validation](VALIDATION.md) | Local checks and GitHub Actions policy |
| [Deployment](deployment/README.md) | Cloudflare, native hosts, rollout and recovery |
| [Targets and domains](DEPLOYMENT_TARGETS.md) | What needs hosting and what still needs assignment |
| [Security](security/README.md) | Source publication and credential scanning |
| [Component contracts](components/) | Authentication/Panel API references and attribution |

## Documentation site

From this directory with Node 24:

```sh
npm ci --no-audit --no-fund
npm test
npm run check
npm run build
```

The build produces a static site in ignored dist/ with a source-commit manifest,
sanitized Markdown and security headers. It requires no backend or paid hosting.
Source CI tests and builds it automatically; publishing it remains a separate
deployment decision. Review content before adding operator-specific instructions.
