# Synthetic product demo

Populate a fresh local Panel through its real APIs with fictional players, servers,
guilds, quests and market listings. This is a development fixture for UI review and
screenshots; it does not connect to a Minecraft server or deploy infrastructure.

Build the backend and frontend as described in [the application guide](../../docs/APPLICATION.md), then run from the repository root:

```sh
node scripts/demo/run.mjs
```

Optional arguments supply the backend binary and built web directory. The runner
binds only `127.0.0.1:18080`, refuses an occupied port, creates a new OS temporary
directory on every launch, generates an ephemeral signing secret and stops its own
processes on Ctrl+C. It never opens an existing operator store or deletes data.
Temporary data remains outside the repository for inspection after shutdown.

Open the printed address. Admin: `admin` / `demo-admin-pass`.
Player: `Alex_Miner` / `demo-pass-1234`. These are deliberately public demo
credentials and must never be used on an externally reachable service.

The seeder requires `VELORA_DEMO=synthetic` and a loopback HTTP endpoint; the runner
sets both. Do not manually point the seeder at an existing installation: it changes
accounts and application settings. The heartbeat simulates server presence.

For the launcher alone, run `npm run dev` inside `launcher`, then open
`http://localhost:1420/?mock=main`. Its in-memory browser fixture needs no backend.
Other supported scenarios are `setup`, `login`, `update` and `progress`.
