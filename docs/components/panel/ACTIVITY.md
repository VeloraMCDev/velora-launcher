> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Platform activity compatibility boundary

`velora-panel-activity` owns neutral metadata INSERT and activity report projection
operations. It imports no Authentication, Experiences or private monorepo package.
It does not open databases, initialize mixed stores or expose an HTTP service.

The host supplies verified username/UUID and time to `record`. Detail sanitization
removes control characters and retains the first 256 Unicode characters. Callers
must never pass passwords/tokens, player chat or command arguments as metadata.
Storage errors propagate; existing host middleware still decides whether a failed
post-response audit is logged, while login failures retain their original ordering.

`list` reads supplied `events`, `server_events` and `game_servers` projections.
Those names describe the interim legacy compatibility boundary; they do not
authorize importing gameplay tables, triggers or migrations. Rows retain original
IDs, nullable values, unrecognized source/kind/detail values and server-name joins.
Reports order by timestamp descending, source and ID descending, with the original
100-row limit and bounded offset. Name matching remains SQLite NOCASE and UUID
matching remains exact. Parameters are bound rather than interpolated.

Live administrator authorization remains the caller's responsibility. The library
does not replicate credentials or admit a caller based on cached account data.
Mixed identity/gameplay deletion, durable Authentication audit projection, report
APIs across separate stores and full Panel service/interface composition remain
incomplete. A cross-domain report must not become a cross-service SQL transaction.

Synthetic projection fixtures verify full serialized response shape, nulls, unknown
payloads, ID preservation, orphan-server join behavior, Unicode sanitation, outage
errors, filtering and bounded pagination. No production records are fixtures.
See [provenance](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/ACTIVITY_PROVENANCE.json).
