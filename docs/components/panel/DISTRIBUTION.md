> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Landing/download records and helpers

velora-panel-distribution owns the neutral hosted-download, FAQ, block and theme
records, original installer/repository/version classifiers and display filename
helper. Opaque extension block options retain their JSON values.

The compatibility host retains private landing defaults/presets, live authorization,
provider/cache behavior, upload/download routes, real stored filenames/URLs and file
lifecycle. These helpers do not access storage or change installer/package/signing
identities. Empty/non-ASCII brand display-name fallback now says Velora; supplied
operator branding, including legacy names, is honored.

    cargo +1.98.1 test -p velora-panel-distribution --locked
    cargo +1.98.1 clippy -p velora-panel-distribution --all-targets --locked -- -D warnings

Both original helper tests are retained; the one empty-brand expected value changes
with the requested display rebrand. Extra tests cover unchanged file identities,
opaque options/default wire fields and rejected repository/installer inputs. Full
landing/service/page and native upgrade acceptance remain open. See provenance.
