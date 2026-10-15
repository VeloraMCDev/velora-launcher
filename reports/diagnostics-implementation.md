# VEL-018 bounded diagnostics implementation

October 12, 2026. Direct checkout; no branches, commits, deployments, native API
changes, dependencies, persisted identities, or unrelated modifications.

## Exclusive implementation files

- `launcher/src/components/CrashDialog.svelte`: replaces raw crash-tail preview
  and copy with safe structured guidance and identical redacted copy/export text;
  retains existing native file/folder/retry actions with a privacy warning.
- `launcher/src/lib/diagnostics.ts`: pure deterministic projection of existing
  snapshots, fixed signature summaries, explicit scope and privacy, bounded input
  inspection and report output. No health or native launch claims.
- `launcher/tests/diagnostics.test.mjs`: executes the production helper against
  synthetic secrets, URLs, paths, IPv4/IPv6, email, controls, oversize data,
  unknown extensions, malformed/empty snapshots and qualified category advice;
  compiles the real dialog and checks its explicit safe sharing wiring. Its actual
  copy/export functions are extracted using the Svelte AST and run in an isolated
  VM with synthetic clipboard/download adapters (not reimplemented handlers).
- `reports/diagnostics-contract.md` and this report: scope, limits and gates.

Root instructions, MONOREPO, BOUNDARIES, VALIDATION, DEPLOYMENT_TARGETS and the
available master-plan completion ledger were read. No deeper AGENTS.md applies
to these five files. Existing dirty checkout changes are outside this work.

## Checks

- From `launcher`, `node --test tests/diagnostics.test.mjs`: **9 passed**, no
  failures or skips. Covers exact preview/copy/export payload agreement,
  no sharing on initialization, awaited clipboard failure, fixed error text,
  empty-crash action guards and object-URL cleanup on export success/failure.
- From `launcher`, `./node_modules/.bin/tsc --noEmit --strict --target ES2022
  --module ESNext --moduleResolution bundler --skipLibCheck src/lib/diagnostics.ts`:
  exit 0. The targeted test also compiles CrashDialog with zero Svelte warnings.
- Whitespace checks on all five exclusive files: clean.
- Checks used locally available Node **25.6.1**, not the documented Node 24;
  rerun under the required CI toolchain before crediting toolchain parity.
- No full build, native run, mounted-browser acceptance, or release was performed.
  All fixtures are synthetic; no player/runtime stores or operator data were read.

The initial test run caught an overly broad assertion matching the CSS `.tail`
selector and an assertion matching a negated deletion statement. The assertion
now targets raw crash-tail access, and mod advice states that report generation
does not modify files. A patch initially used root-relative paths from the launcher
cwd and was rejected without writing; it was reapplied from the root. Final
targeted runs pass. The completion ledger was not edited outside the write scope;
this report supplies source evidence for checkpoints 1–3, not native checkpoint 4.

## Native feature acceptance gate (open)

Run synthetic crash/preparation fixtures in each supported native desktop
webview. Verify preview/category accessibility, clipboard permission success and
denial, exact copied text, local export filename/content, download cancellation,
and object-URL cleanup. Confirm no upload occurs and original sensitive strings
never appear in report preview/copy/export. Revalidate original file/folder
actions and retry request compatibility without treating them as proof of launch.
Native game/loader, OS permissions, credential store, installed-data/updater,
packaging and release acceptance remain separate manual gates. Source/helper
tests do not close checkpoint 4 or justify overall release-readiness claims.
