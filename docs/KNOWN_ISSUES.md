# Known limitations

Keep release claims tied to the actual validated runtime. Current deployment gates
are recorded in [deployment targets](DEPLOYMENT_TARGETS.md).

During the 2026-10-08 synthetic Panel screenshot session, the profile-post write
endpoint returned a SQLite error because user_posts is a view. Core demo data
(accounts, quests, guilds, friends, market, progression and server presence) seeded
successfully. Profile posts are not represented as working in the new screenshots;
their write path requires a separate runtime fix and regression test.

The fixture also encountered SQLite busy transaction errors during guild creation;
its seeder retries only explicit database-lock failures with bounded backoff.
This fixture behavior does not resolve production database concurrency concerns.

Desktop browser previews do not exercise native Minecraft launching, OS credential
storage, installer upgrades or updater signing. Exact-image Docker acceptance,
production source-policy registration, host/domain assignment and restore testing
remain independent gates. No new production acceptance is claimed by README cleanup.
