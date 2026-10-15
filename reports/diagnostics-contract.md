# VEL-018 checkpoints 1–3: bounded user-support diagnostics

October 12, 2026. This implements the user's approved existing feature direction,
not a new health-monitoring feature. The available master-plan ledger is
`reports/completion-checklist.md` (VEL-018); no separate master-plan attachment or
file was available in this checkout. Checkpoint 4 remains native feature acceptance.

## Inputs and privacy boundary

The pure `createDiagnosticReport` helper receives existing crash/preparation
snapshots only. Its allowlist reads crash `code` and `tail`, and preparation
`state`, `stage`, and `message`. It never traverses unknown extension fields.
The dialog passes `app.crash` and `app.launch`, not bootstrap, account, session,
settings, manifest, console history, or environment data.

Only bounded integer exit codes, exact known state/stage enums, and fixed
signature/category/advice strings enter the report. Original free text is never
echoed: recognized error signatures become fixed summaries; every other entry
becomes `[redacted unrecognized text]`. This deliberately sacrifices detailed
stack traces rather than relying on incomplete secret-pattern masking. URLs,
credentials, opaque tokens, usernames, player paths, server addresses, IPs,
emails, and control characters cannot survive as original free text, even when
they occur beside a recognized signature. No account, instance, run or operation
IDs, crash-report paths, dynamic filenames, timestamps, or arbitrary fields are
serialized. Existing APIs and persistent identities remain unchanged.

## Bounds and interpretation

- Inspect at most the last 40 crash entries and first 512 UTF-16 code units of
  each entry; inspect at most 512 units of the preparation message.
- Replace control/format characters before matching; never coerce non-string entries.
- Emit at most 8192 UTF-16 units. Oversized input has an explicit omission flag;
  the output ceiling can also cut the final summaries/footer.
- Categories have a fixed order: memory, Java version, memory configuration,
  mod/loader, network, file access; unmatched/empty data yields unknown guidance.
- Signatures are observations, not established causes. They may appear in quoted
  or misleading log text; advice is qualified and never deletes files or promises
  a fix, compatibility, available memory, connectivity, health, or launch success.
- Preparation state is current global UI context, not a correlated crash-run
  history. The report explicitly states that limitation; it does not invent a link.

## User control and compatibility

The dialog previews exactly the report used by Copy report and Export report.
Neither action runs on render or in an effect. Copy awaits clipboard completion
before success feedback and uses fixed failure feedback. Export creates a local
text Blob with fixed filename `velora-support-diagnostics.txt`, triggers a browser
download only on click and revokes the object URL. Download completion is not
claimed. There is no automatic upload, network request, native diagnostics
command, filesystem read, system/environment probe, or new dependency.

Existing user-triggered Crash report, Logs and Try again actions retain their
native APIs. The dialog warns that original files may contain private information;
they are not included in the support report. Native retries remain requests,
not verified launches. Local source tests cannot establish webview clipboard,
download destination/cancellation, native file opening, or game launch acceptance.
