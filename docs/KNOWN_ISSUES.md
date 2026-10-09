# Known limitations

Keep release claims tied to the actual validated runtime. Current deployment gates
are recorded in [deployment targets](DEPLOYMENT_TARGETS.md).

During the 2026-10-08 synthetic Panel screenshot session, the profile-post write
endpoint returned a SQLite error because user_posts is a view. Core demo data
(accounts, quests, guilds, friends, market, progression and server presence) seeded
successfully. Profile posts are not represented as working in the new screenshots;
the screenshots remain historical fixtures. The write path now uses the shared
platform database. A persistent-instance regression verifies cross-instance posts,
likes, owner-only deletion and reopening stores; all 13 instance/social tests pass.

The fixture also encountered SQLite busy transaction errors during guild creation;
its seeder retries only explicit database-lock failures with bounded backoff.
This fixture behavior does not resolve production database concurrency concerns.

Desktop browser previews do not exercise native Minecraft launching, OS credential
storage, installer upgrades or updater signing. Exact-image Docker acceptance,
production source-policy registration, host/domain assignment and restore testing
remain independent gates. The manual Panel image workflow now checks cold restore
of disposable stores, sessions, signing identity and instance data before publishing.
Adding that gate does not claim a completed image run or production rollout.
