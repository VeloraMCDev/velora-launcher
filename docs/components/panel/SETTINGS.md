> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Settings and KV boundary

velora-panel-settings owns the original settings record, username-rule defaults,
normalization/validation, branding-name validation and supplied-pool KV reads/writes.
The auth model is generic: compatibility hosts retain their existing model and
defaults instead of replacing them with SDK defaults. JSON fields and malformed
stored-value fallback remain compatible.

The host retains live administrator guards, exact pool/scope selection, secret
masking/flags, environment precedence, derived auth URL clearing and private
Experience branding resolution. The stored record can contain a provider key;
callers must keep the existing response projection and credential protections.

Empty/absent key means keep, '-' clears, other keys trim/replace. Username entries
normalize before the existing limits and validation; public URLs retain original
slash trimming and prefix checks. Serialization errors happen before SQL and map
separately from database failures. No database is opened or schema initialized.

    cargo +1.98.1 test -p velora-panel-settings --locked
    cargo +1.98.1 clippy -p velora-panel-settings --all-targets --locked -- -D warnings

Synthetic custom-auth defaults, two pools, unavailable stores, malformed JSON,
secret replacement, validation order/limits and failed serialization exercise the
actual library. Complete settings routes/frontend/service composition and release
acceptance remain migration work. See SETTINGS_PROVENANCE.json.
