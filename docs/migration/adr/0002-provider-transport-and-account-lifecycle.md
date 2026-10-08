# ADR 0002: provider transport and account lifecycle

Date: 2026-10-05. Status: implemented library boundary; service cutover deferred.
Applies to D7 and AUTH-04/PANEL-07.

## Context

The legacy connections route combines protected operator settings, generic SMTP
and Discord HTTP transport with identity reset/link workflows and live route
authorization. Moving the complete file would mix ownership and make provider
configuration a startup dependency between services.

## Implemented decision

Panel owns the generic communications library: settings validation/masking,
secret-preserving updates, plain-text/alternative-HTML mail and Discord token/profile
transport. It uses public dependencies and contains no gameplay, credential SQL,
bulk audience selection or private templates. Authentication retains account
allocation, linking, reset tokens and one-use callback state.

Panel also owns generic mail layout and persisted template shape/validation.
Rendering consumes resolved text and a trusted HTML footer, preserving the frozen
HTML bytes and existing formatting/link policy. Private rank/guild/level variables,
starter catalogs, recipients and opt-out/queue storage remain host contributions.
Bulk replacement keeps its distinct ID policy, count limit and error precedence;
individual create endpoints retain their ID-length rule and conflict ordering.

The compatibility host loads live protected settings, enforces administrator/user
guards and resolves the callback origin before transport. It supplies its existing
HTTP client, retaining timeout/redirect configuration. Error mapping preserves
configuration/provider rejection as 400 and transport/delivery failure as 502.
Invalid settings are rejected before persistence; public responses contain flags,
and administrator responses mask stored secrets.

SMTP remains one acceptance attempt against the existing relay. The caller retains
recipient scope, queue and retry behavior. Acceptance does not certify inbox
delivery. Token requests precede bearer-authenticated profile requests; malformed
or rejected tokens stop before profile lookup. Unknown profile fields remain intact.

## Evidence and limits

Eight independent communications tests cover protected settings, exact masked
shape, secret update/validation precedence, synthetic MIME messages, loopback HTTP
form/bearer exchange and provider rejection/JSON/transport failures. Three activity
tests, formatting and strict clippy also pass in the independent public workspace.
Frozen-source/output hashes and guarded host snapshot checks record the extraction.
No live provider request, production data import or visibility change is required.

Three template tests cover wire defaults, unknown placeholders, Unicode character
limits, ID trimming/case/duplicates and validation order. Two renderer tests cover
exact synthetic frozen-output bytes, escaping, link policy and trusted footer
handling. The independent public workspace now passes sixteen tests in total.

Standalone service configuration/secret delivery, authenticated internal calls,
durable retries, complete bulk/admin separation, cross-service reset/callback
integration and deployment/upgrade acceptance remain unresolved. Do not replace
these with no-op transport or mark the parent acceptance tasks complete.
