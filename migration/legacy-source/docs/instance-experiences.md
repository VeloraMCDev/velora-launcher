# Instance experiences

**SCOPENET owns the platform. Each instance owns the experience.**

Administrators start at `#/instances`, enter an instance's control center, and configure its systems under `#/instance/<id>/<page>`. The version/modpack editor remains available under `installation`. Platform accounts, groups, capes, launcher publishing, authentication and site settings stay accessible from the platform navigation.

The launcher and browser player panel select the same instance experience. Switching instances resets the current experience page, reloads its data, and applies its branding. Existing SMP pages remain available; enabled capabilities determine which pages and API endpoints can be used. Disabling a capability retains its data.

## Ownership audit

| Platform | Experience |
| --- | --- |
| Accounts, credentials, authlib/Yggdrasil, skins and player identity | Economy, casino, orders, contracts and guild banks |
| Groups and shared membership | Guilds, claims, maps and map actions |
| Friends, messages, profiles and social posts | Levels, XP curves, quests, achievements, rewards and collections |
| Installation metadata, modpack/file distribution, Java and game launch | Content Studio, resource packs, commands, chat and companion layouts |
| Launcher/mobile updates, OAuth/email delivery configuration, global branding fallback | Experience branding, news, navigation, widgets and module configuration |
| API/auth infrastructure and server credential ownership registry | Connected server runtime, instance integration settings, group role mappings and scheduled jobs |
| Shared social presence, aggregated player activity reports and platform account/admin audit | Gameplay event history and progression displays |

`crates/shared/src/experience.rs` defines the wire contract. `shared/experience/index.ts` defines capabilities, page associations, navigation and SMP/Frontiers presets. `panel/server/src/experience.rs` resolves request ownership and manages instance stores. Existing domain handlers receive `RequestState`, so their transactions run against the selected store without scattering instance predicates across hundreds of gameplay queries. Authentication always uses the platform pool.

## Storage and migration

The platform keeps `data/panel.db`. Gameplay databases and map images live in `data/experiences/<instance-id>/`. Each scoped SQLite connection attaches the platform database and exposes read-only identity views. Credentials are never copied; group integration mappings overlay shared groups with instance-owned values. Server IDs remain globally allocated, while runtime rows are owned by their instance. Shared presence carries only server status and online players. Shared player reports aggregate current instance stores without writing a second gameplay history to the platform.

On first access, each experience transactionally adopts its existing server/instance/guild-owned rows. Ambiguous legacy network-wide progression, settings and definitions go to the **oldest instance**, ordered by creation time and ID, once. Previously unassigned servers are assigned to that primary instance. Map images follow server ownership; the original images and gameplay history are retained. Already-earned history is imported without running reward/XP triggers again. A marker prevents replay on restart. Existing instances default to the full SMP capability set.

Back up the data directory before an upgrade. Ensure the oldest instance is the intended owner of legacy global systems before first use; these historical rows have no information from which another owner could be inferred. Installations retain their IDs, files, access rules and revisions. Instance deletion stops its store's workers and removes its server credentials, while preserving its gameplay directory; new instances cannot reuse that retained directory's ID. Account deletion cleans every active experience before deleting shared identity, with separate transactions per store and retryable cleanup if a store fails.

The platform retains historical gameplay tables for migration compatibility. New experience requests never read them as a global gameplay store. In-memory legacy unit fixtures use the original single-store mode; production experience isolation requires the persistent platform database.

## API context

- Browser requests use `X-SCOPENET-Instance: <id>`; native launcher requests use the selected instance saved by `select_experience`.
- `?instance=<id>` is available to integrations that cannot set headers. Conflicting header/query context is rejected.
- Server API ownership is inferred from its bearer credential. Supplying another instance is rejected.
- With one instance, older clients can omit context. With multiple instances, gameplay calls without context return 400.
- Hidden/disabled instances are checked against the shared user and groups. Disabled feature endpoints return 403.
- Platform auth, manifest, identity/social, installation and publishing APIs need no experience context.
- `PUT /api/admin/instances/<id>/experience` saves validated presentation/capability configuration.
- `GET /api/admin/instance-groups` exposes shared groups with this instance's integration mappings.
- Signed map tile URLs carry instance ownership. Legacy map URLs resolve ownership from the server ID. Generated resource pack snapshots resolve ownership from their checksum, preserving Minecraft's downloads without custom headers.
- Discord interaction URLs for multiple instances must include `?instance=<id>`; shared OAuth callbacks stay on the platform.

## Different experiences and custom components

The SMP preset keeps existing economy, casino, guild, progression, quests and reward pages. The Frontiers preset enables maps, content, events and companion support, and supplies its own overview, navigation and settlement/resource widget. These presets configure presentation and systems; they do not implement a city simulation in the launcher.

Register a built-in Svelte component in `shared/experience/components.ts`. Both the launcher and player panel render that component by its widget ID. Components receive the owning `experience` and `widget`; manifests contain data, never executable remote code. The existing `text` and `links` widgets require no custom component. Page labels/order are configurable within the installed page registry. Extending that registry adds new pages using the same capability contract.

A game integration can publish live module snapshots through:

```http
PUT /api/server/v1/experience/modules/frontiers
Authorization: Bearer <server credential>
Content-Type: application/json

{"settlements":[{"name":"Oakvale","citizens":12}],"resources":[{"name":"Timber","amount":240}]}
```

The module must be registered in the owning instance's `experience.modules`. Snapshots must be objects up to 32 KB; they are stored separately from admin configuration and included in that instance's manifest. The Frontiers widget accepts settlement/resource summary arrays. Simulation, resource accounting, permissions and persistence of the underlying gameplay remain the modpack/server module's responsibility. Public instance manifests are public presentation data: do not publish secrets or private gameplay details in modules or widgets.

## Verification

`panel/server/tests/instance_experiences.rs` uses real persistent databases to cover independent settings/progression, shared identity without copied credentials, disabled API capabilities, server credential ownership, live module snapshots, separate integration mappings, global game authentication, restart-safe migration, map preservation, live account access checks, capability-aware tasks and shared activity reports without duplicated stats. `node --test shared/experience/index.test.mjs` checks capability/navigation behavior. Run the existing Rust tests, both Svelte checks/builds and runes checks alongside these tests.
