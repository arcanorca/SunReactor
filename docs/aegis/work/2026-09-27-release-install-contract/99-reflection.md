# Release and installer repair reflection

- The published `v0.12.0` binaries reported `0.1.0`, omitted the expected systemd unit, and the musl path had previously failed its smoke test. The release producer, archive contract, installer validation/rollback, and regression suite were aligned before publishing `v0.12.1`.
- The release workflow prevented stale or malformed artifacts from being published. Windows CI then exposed test assumptions hidden behind earlier Clippy failures; those were corrected at the test/platform boundary, and the TUI timezone fallback was consolidated with the existing IANA resolver contract.
- `v0.12.1` was rebuilt for all four Linux targets, published from a tag whose commit passed CI, and its public files matched the qualification checksums and contract.
- Bounded follow-up: install the public release on the user's own host and check its local configuration/hardware behavior if that was part of the original symptom.
