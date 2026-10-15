# Prior current-host native observations: evidence audit

Audit recorded: **2026-10-15T04:14:16Z**, equivalent to
**2026-10-15 00:14:16 America/New_York (EDT, UTC−04:00)**.
Scope: independent documentary audit of prior synthetic macOS evidence only;
**not an independent GUI replay or new native acceptance run**.
Ledger remains **62/76 (81.58%)**, with no gate advancement.

## Sources and attribution

Read in full:

- `reports/native-acceptance-implementation.md`.
- `reports/native-acceptance-isolation-review.md`, including the historical
  HOLD, subsequent bounded guard acceptance, and conditional developer-container
  opinion. Those dispositions are not GUI acceptance results.
- `/tmp/velora-revitalization-20261014/native-container-gui.log`.
- `requests.ndjson` beneath
  `/private/var/folders/nc/8f6g1lzn2h3931lhvc1xylt40000gn/T/velora-native-5594843b-7516-42db-a015-0905a1ca279f`.
- The lead's prior-session summary supplied with this audit request.

This reviewer read the two logs directly. GUI behavior, binary-copy equality,
real Tauri IPC attribution, and process observations below are **lead-reported**
unless explicitly identified as log evidence. No native/GUI, fixture, Vite,
helper, or game process was launched or rerun for this audit. No real stores,
credentials, or player data were inspected. No fresh process-list or binary
inspection is claimed. Only this report is authored by this reviewer.

SHA-256 of the evidence files read at audit time:

```text
1cb568f883f204969b56145ca48694613861150b2a690ea6be8e5804322a27c6  native-container-gui.log
14db4f824ee0b975c6a6a9b314c6831213bf5075b87ef717f8107d7b67862ba6  requests.ndjson
```

## Time basis and directly audited log evidence

Log timestamps ending in `Z` are UTC, not New York local time. The captured
request window is **2026-10-15 02:08:06.175–02:12:48.336 UTC**, equivalent to
**2026-10-14 22:08:06.175–22:12:48.336 EDT**. Thus the October 14 local-session
date and October 15 UTC entries are consistent; the directory's `20261014`
label is not a UTC event timestamp. The later audit itself falls on October 15
in both zones.

The request log contains **21 entries**: one allowlisted manifest route, four
allowlisted synthetic-instance routes, and sixteen `/unmatched` entries.
Only timestamps and route labels are present. There are no request methods,
response statuses, response bodies, cancellation acknowledgements, process
exit records, or per-action correlation IDs. `/unmatched` intentionally loses
the actual path: it cannot identify authentication, telemetry, friends, or
other specific operations.

| UTC on October 15 | New York EDT on October 14 | Directly recorded evidence |
| --- | --- | --- |
| 02:08:06.211 | 22:08:06.211 | `/api/v1/launcher/manifest` request. |
| 02:08:30.822 | 22:08:30.822 | `/api/v1/launcher/instances/synthetic-native-vanilla` request. |
| 02:08:30.832049 | 22:08:30.832049 | Native `commands::launch` reports `launch failed: panel request failed`. |
| 02:08:53.840 | 22:08:53.840 | Another synthetic-instance request. |
| 02:08:58.844655 | 22:08:58.844655 | Second native launch-failure entry; timing is consistent with a five-second delay, not proof of a cancel. |
| 02:09:53.786 | 22:09:53.786 | Another synthetic-instance request; no cancellation result recorded. |
| 02:12:34.148 | 22:12:34.148 | Final recorded synthetic-instance request. |
| 02:12:48.336 | 22:12:48.336 | Final recorded request is `/unmatched`; not a quit or exit acknowledgement. |

The native log has **eight lines: two launch errors and six telemetry warnings**.
Warnings say `activity report failed: panel request failed`, at UTC
02:08:30.832047, 02:08:30.833144, 02:08:53.842077, 02:08:58.845883,
02:09:53.787621, and 02:12:34.156964. Near-simultaneous lines are not strictly
timestamp-ordered in the file. No stack trace, HTTP status, readiness signal,
native cancellation acknowledgement, or process exit status appears in it.

## Prior lead-reported native observations

These observations are preserved as supplied, not promoted to independently
replayed findings. Exact GUI-action timestamps were not supplied; the sparse
logs cannot assign every action a reliable timestamp.

| Area | Prior observation | Evidence limit / disposition |
| --- | --- | --- |
| Executable and IPC | Actual temporary native `.app` container used a byte-identical feature debug binary; interactions used real Tauri IPC. | The isolation report records the approved source-binary hash below; copy equality and actual IPC use are lead-reported. HTTP route logs alone do not establish either. Not release/package acceptance. |
| Initial bare launch | Vite was absent; the UI-did-not-signal-ready warning occurred and the owned process was stopped with SIGTERM. | Failed readiness attempt, **not successful native GUI startup**. This attempt is reported in the isolation addendum, not established by the container log. |
| Verify/repair failure | Controlled 503 produced **Preparation failed**; safe details redacted raw errors. | Bounded visible failure/redaction observation. Logs corroborate generic backend failures but contain no HTTP status or rendered UI, so do not independently prove 503 or redaction. |
| Five-second retry | Stale cancellation attempt was **not successful**. | Do not count cancellation acceptance; the delayed failure log is not evidence of a successful cancel. |
| Thirty-second retry | Cancel was requested; the operation ended afterward. | **UNCONFIRMED** cancellation: temporal ordering only, explicitly not a native cancellation acknowledgement. No cause or backend cancellation success established. |
| Offline account dialog | Background was absent from the accessibility (AX) tree/inert; Tab showed visible focus on username; Escape restored the background. | Narrow lead-reported modal/focus observation only. No submit and no real authentication; no account persistence, credential-store, or auth-flow acceptance. |
| Friends | Empty state only. | No populated list, messaging, presence, or failure/recovery acceptance. |
| Busy close | During delayed repair, header Close led to CUA reporting **App quit**; the exact owned process was subsequently absent. | Exit status is unknown. No native quit acknowledgement, proven graceful shutdown, helper/game cleanup, or process-matrix acceptance. |
| Final process observation | Lead's current `ps` observation found no native fixture, Vite, or owned native process. | Supplied observation, not independently repeated or exactly timestamped here. Absence later does not prove how shutdown occurred. |
| Screenshots | No saved screenshot artifacts. | No retained screenshot evidence for independent visual verification. |

Lead-reported executable SHA-256 (also recorded for the source executable in
the isolation report):

```text
9b5e5fa9601e8c8e76a0fa083a1b6e629538bd861d093d218ac51e4a5125a2ab
```

## Privacy exception and unresolved gates

The implementation/isolation reports describe synthetic settings with
`send_stats=false`. Despite that, the prior native log directly records six
activity-report failure warnings. The lead is investigating the privacy source.
**This is unresolved, not an accepted privacy pass or a demonstrated fix.**
Warnings establish attempted activity-report work reaching a failure path;
they do not reveal payloads, destinations, successful delivery, or why the
setting did not suppress that path. The route-only `/unmatched` entries cannot
resolve those questions. No external delivery or real-data disclosure is
established by these artifacts, and neither is complete network containment.

The source review's fallback-only credential routing, static tree guards, and
pre-build incognito configuration are bounded source/test evidence, not a
runtime trace proving zero installed-store reads, zero OS-keychain access, or
all WebKit/OS effects. The conditional container opinion expressly does not
guarantee absence of system-managed registration/cache/diagnostic artifacts.
This audit neither reopens real stores to test those claims nor certifies them.

Remaining gates include native acknowledged cancellation, known exit status
and graceful shutdown, helper/game cleanup and process matrix, privacy-source
resolution, populated social flows, real authentication/credential-store
acceptance, successful install/game behavior, runtime storage isolation,
installed upgrades/recovery, supported-OS coverage, signing, and release
packaging. Nothing here certifies those gates or authorizes a rerun, deployment,
or release. **No change to 62/76.**
