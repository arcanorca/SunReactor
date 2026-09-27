# systemd user-unit compatibility

The shipped file is a template. `@BINDIR@` must be replaced once with an
absolute path; `ExecStart` and `ExecReload` therefore cannot drift.

| Directive | Classification | Reason |
| --- | --- | --- |
| `NoNewPrivileges` | `SUPPORTED_AND_SAFE` | Does not prevent DDC, sysfs, config, helper execution, or IPC on the tested user manager. |
| `PrivateTmp` | `SUPPORTED_AND_SAFE` | The daemon does not use host `/tmp` for its protocol or persistent state. |
| `UMask=0077` | `SUPPORTED_AND_SAFE` | Protects user-owned socket/config/state files. |
| `LockPersonality` | `SUPPORTED_AND_SAFE` | No runtime personality changes are required. |
| `MemoryDenyWriteExecute` | `SUPPORTED_AND_SAFE` | The Rust daemon and its helpers do not require JIT mappings. |
| `RestrictRealtime` | `SUPPORTED_AND_SAFE` | Brightness control does not require realtime scheduling. |
| `RestrictSUIDSGID` | `SUPPORTED_AND_SAFE` | The daemon does not create setuid/setgid files. |
| `SystemCallArchitectures=native` | `SUPPORTED_AND_SAFE` | Release binaries and helpers use the native ABI. |
| `ProtectKernel*` | `UNVERIFIED` | Removed after a reported Ubuntu 22.04 user-manager spawn failure; there is not yet a directive-by-directive Ubuntu hardware trace proving one exact culprit. |
| `ProtectHostname` | `UNVERIFIED` | Namespace setup is unnecessary for this per-user daemon and was part of the failing hardening set. |
| `RestrictNamespaces` | `UNVERIFIED` | It was part of the reported failing set; the exact Ubuntu 22.04 directive-level failure was not captured, and it does not replace device permissions. |
| capability bounding directives | `REDUNDANT` | A user service has no ambient privileged device capability; udev/logind and file permissions govern access. |

CI renders the template and runs `systemd-analyze --user verify` on Ubuntu
22.04, Ubuntu 24.04, and the current Ubuntu runner. A real daemon startup is
still required before describing a particular installed unit as validated.
