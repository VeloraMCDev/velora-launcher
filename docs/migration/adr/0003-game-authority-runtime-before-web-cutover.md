# ADR 0003: game authority runtime before web cutover

Date: 2026-10-05. Status: implemented standalone game runtime; full service cutover deferred.
Applies to AUTH-08 and the remaining AUTH-09/10 composition gates.

## Context

Owned Yggdrasil HTTP, identity/session/cosmetic storage, schema/import, key loaders
and configuration can form a working process independently of private gameplay.
Web account mutation still needs live presence and durable audit/projection decisions.
Mounting those handlers with no-op ports would change admission and rollback behavior.

## Decision implemented

Assemble `velora-auth-server` with the real existing Yggdrasil and texture routes,
owned SQLite schema, socket-peer proxy resolution, operator origin/name and protected
signing/bootstrap files. New storage initializes only an owned empty store; existing
stores validate read-only before writable startup. Mixed/damaged schemas and signing
material fail without implicit repair or rotation.

An OS-held canonical database lock excludes another instance of this new service.
It does not coordinate the old application: production import/quiescence and the
single-writer cutover remain required. Conditional first-admin insertion retains
existing users, UUIDs and credentials, skipping credential-file reads when any
administrator already exists. No default password or private hosted origin is added.

Health distinguishes responsive process from live owned-store readiness. Shutdown
stops readiness, drains requests, closes SQLite and releases the held writer lock.
Incoming host headers cannot control signed public origins. Game admission uses
existing live account/session checks and preserves errors during store outages.

Only supported game routes are mounted. Web/admin, reset/Discord, presence/audit,
Panel projections, deletion and a compatible gateway are not silently replaced by
fallbacks. Existing compatibility-host routes and credential writers remain unchanged.

## Evidence and remaining acceptance

Local Windows tests exercise the real TCP listener, game login/join, signed profile
verification, spoofed-IP rejection, restart credential/key/UUID/token continuity,
duplicate writer rejection, mixed-store/damaged-key refusal, live database outages,
CLI errors and graceful runtime shutdown. Independent Linux CI additionally exercises
the actual executable with SIGTERM and writer-lock release. Runtime composition and
library provenance identify the public files and dependency baseline.

Full web/service/gateway composition, operational process-manager/container profiles,
complete fresh-install/import/upgrade/rollback, provider delivery and full private
feature parity remain incomplete. This runtime alone does not close AUTH-08/09/10
or authorize deployment, publication, visibility changes or production migration.
