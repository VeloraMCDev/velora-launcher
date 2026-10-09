# Windows upgrade identity

This template comes from Tauri CLI 2.12.0, commit
`447fa9f3f993fe77724189e355078b38ce20baea`,
[`installer.nsi`](https://github.com/tauri-apps/tauri/blob/447fa9f3f993fe77724189e355078b38ce20baea/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi).
It is used under its MIT license, retained in LICENSE-MIT.
The unmodified source SHA256 is
`dabed59013b1d78b879a1a85bc7f2eed2993b33a9a90cdabe5946de3d3950597`.

Only UNINSTKEY and MANUPRODUCTKEY differ: they retain the installed registry
identities while PRODUCTNAME, publisher and executable use Velora. This preserves
previous-install detection and the installed directory. The source identity check
reverses those two changes and verifies the upstream hash and locked CLI version.
Review this template when upgrading Tauri CLI.

The manual Windows release job installs the verified 1.3.0 installer and upgrades
it with the new installer on its disposable runner. It checks the registry display
name/version/publisher, installed executable and preserved synthetic user data.
