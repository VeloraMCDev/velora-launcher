# Monorepo consolidation record

`source-map.json` pins the seven former owner heads and source/target hashes for
265 runtime-package files. Source mapping also records the canonical replacement
for Panel's HTTP snapshot; its four modules matched `shared/http` after line-ending
normalization. No mapped runtime file remains missing.

Infra and Docs were imported from tracked files only. Generated output, dependency
directories, local keys, operator state and `.git` directories were excluded.
Component introductions, provenance and legal notices were retained. Independent
Java package build files and tests were restored; builders identify the canonical
repository. SDK snapshots used by private Java/UI builds were compared before
replacing them with direct references; historical manifests remain in
`../superseded-snapshots`.

Reviewed differences are Cargo package/workspace/dependency metadata, auth module
declaration ordering, launcher test branding/formatting/import aliases, canonical
login-client imports, independent Gradle project paths, artifact source identity,
and maintained documentation. Private gameplay and installed identities remain
unchanged. Owner README links are pinned to historical GitHub source when the
original relative target belongs to a retired repository.

Full histories remain in the owner repos and ignored local bundles. These bundles
are local recovery custody, not a public asset or an independent durable backup.
Mark former repositories deprecated and for archiving. Actual deletion requires
durable history custody and review of pinned consumers, release assets and live
control-plane source policies. Do not merge unrelated histories merely to delete
old repositories: doing so increases publication exposure.

Use [the publication audit](../../docs/security/PUBLICATION_AUDIT.md) before any
public export. Frozen legacy source is reference only; it was not rewritten.
