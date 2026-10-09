# Shared frontend modules

The neutral browser transport lives in [http](http/README.md). Map, board, casino,
commands and experience modules provide locally compiled UI shared by the complete
Panel and launcher. Gameplay modules stay outside reusable platform SDK boundaries.
The complete source is public; directory names and licenses retain compatibility.

Run the HTTP client's tests from shared/http after npm ci. Validate shared Svelte
changes with the affected Panel/launcher check, check:runes and build commands.
See [segment boundaries](../docs/BOUNDARIES.md) and [validation](../docs/VALIDATION.md).
Libraries do not require independent hosting or domains.
