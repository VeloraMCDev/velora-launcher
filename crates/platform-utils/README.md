# velora-platform-utils

Maintained Rust platform segment in the root Cargo workspace.

```sh
cargo test --locked -j 1 -p velora-platform-utils
```

Run from the repository root. Reusable libraries must not depend on private gameplay or the complete Panel host. See ../../docs/BOUNDARIES.md and docs/components for component contracts and source attribution.

Downloads verify positive declared sizes and any supplied SHA-1 before replacing
the destination. A size of zero (including an omitted size) remains unspecified,
matching the existing metadata contract. Validation checks bytes written, not an
HTTP transfer header. Failed size validation preserves the previous destination;
partial-file cleanup failures are returned as errors. This is per-file validation,
not transactional batch installation or crash-durable promotion.

Fresh downloads require a full `200 OK` response, reject a body chunk that would
exceed a positive declared size before writing it, and use exclusively created
per-attempt sibling temporaries. Stream/verification/publication
failures remove that attempt's temporary and preserve the prior destination;
cleanup errors retain the original failure context. Zero/omitted sizes and optional
digests keep their established meanings. Zero/omitted sizes now have an 8 GiB
per-attempt ceiling even without a digest; callers can lower it through
`download_one_with_limit`. A file-owning drop guard closes and removes a partial
when the download future is cancelled. Cleanup failure during abort is logged;
ordinary errors return cleanup context. Chunk writes and publication are synchronous
to avoid detached writers, so slow-storage runtime latency still needs profiling.

The shared production client follows at most 10 HTTP(S) redirects, rejects embedded
URL credentials/non-HTTP schemes and refuses downgrade after any HTTPS hop.
Initial/final artifact URLs are checked too; custom clients retain their own
intermediate redirect policy. Configured private/local HTTP remains supported.
No resume, native OS/hostile-directory/parent-fsync or interprocess publication
acceptance is claimed. The separately versioned pack utility snapshot is unchanged.
