> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Communications boundary

`velora-panel-communications` contains generic administrator connection settings,
plain-text/alternative-HTML mail construction and Resend SMTP delivery, and Discord
OAuth token/profile transport. It depends only on public Rust packages.

The `render` module supplies the existing generic branded HTML layout. Input body
and brand text are escaped; callers supply an already trusted HTML footer. The
renderer preserves the existing link/button, heading/list and bold behavior,
including single-star emphasis remaining literal. It does not select recipients,
resolve placeholders or sanitize a caller's arbitrary HTML footer. Synthetic output
fixtures generated from the frozen renderer assert exact HTML byte compatibility.

The `templates` module preserves the persisted four-field template shape, Unicode
character limits and bulk replacement validation. Bulk replacement trims IDs,
rejects duplicates case-sensitively and accepts at most 50 templates; it retains
the legacy absence of an individual ID-length bound. Individual create endpoints
retain their separate 50-character ID rule and conflict/storage ordering in the
host. Empty stored catalogs, defaults, opt-outs and queued delivery remain caller
policy. Private rank/guild/level variable lookup and starter catalogs are excluded.

Protected settings retain their existing JSON fields. Empty secret input preserves
the stored value, `-` clears it, and other values replace it after trimming. Public
responses expose availability flags and administrator responses mask secrets.
Validation runs before storage; the caller owns authorization and persistence.

Mail uses the existing SMTP relay and credentials, performs one acceptance attempt,
and propagates configuration/transport errors. SMTP acceptance does not certify inbox
delivery. Recipient selection, reset templates, queues, retry policy and private
gameplay notifications belong to callers. Tests build synthetic MIME messages and
reject invalid configuration without contacting a provider.

Discord transport receives the caller's HTTP client and callback URL. It preserves
form encoding, request order, bearer profile lookup, unknown profile fields and
the existing 400/502 error distinctions. Loopback tests cover successful exchanges,
rejections, malformed responses, missing tokens and transport failures.

Authentication owns account allocation, linking, password-reset tokens and callback
state. The compatibility host still supplies live settings, authorization and its
HTTP client. This crate is a library; standalone Panel service and secret storage,
service-to-service authentication and separate-service integration remain pending.
