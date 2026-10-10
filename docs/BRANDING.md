# Velora branding and compatible installs

Desktop packages, publisher names, player-facing text and maintained package names
use Velora. Launcher 1.3.1 changes the macOS app bundle itself to `Velora Launcher.app`;
renaming a download filename alone does not change the installed application.
The Debian package declares that it replaces the previous `scopenet-launcher`
package; native release acceptance installs 1.3.0 first and verifies the upgrade.

Technical identities used by existing installs remain compatible: the desktop app
and credential service use `net.scopenet.launcher`, mobile apps use
`net.scopenet.player`. The Velora Core Minecraft mod uses Velora names; files and permission nodes created under the SCOPENET names are carried over or still honoured (see
[VELORA_CORE.md](VELORA_CORE.md)). Persisted panel configuration keys retain their existing values. Old environment
variables and HTTP instance headers remain accepted. Source provenance and frozen
compatibility fixtures retain their original attribution and names.

## macOS Keychain

The launcher reads a Keychain entry containing the encryption key for its local saved
sign-in tokens. It does not request access to unrelated passwords. The system dialog
requests the login keychain password, usually the Mac login password. Apple explains
[Allow, Always Allow and Deny](https://support.apple.com/en-gb/guide/mac-help/kychn002/26/mac/26).

Older builds tried to write a replacement key after a denied read, causing a second
prompt. New builds stop on denial and preserve the encrypted credentials. A working
existing fallback key is reused without repeated Keychain requests; an unavailable
or incorrect key cannot overwrite the saved encrypted file. Only an absent entry
with no existing encrypted store may create a new credential identity.

Allow access only for an installation you trust. Always Allow can remember access
for that build; a changed application or a locked keychain can require authorization
again. Native upgrade acceptance remains a device check, particularly for unsigned
or locally signed macOS builds.
