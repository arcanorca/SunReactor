# Real-hardware regression evidence

## Owner Ubuntu 22.04 / MSI external monitor

Status: **NOT TESTED** for this change set.

The development host used for the audit is not the owner's Ubuntu 22.04
machine and has no MSI DDC/CI monitor attached. No brightness write was made.
The permanent MSI/BOE parser inputs under `tests/fixtures/ddcutil/` are redacted
reconstructions of the reported output shape, not a claim that current real
hardware passed.

Before a release is promoted, run the restore-safe procedure documented in
this task on the owner machine and append dated evidence here: original and
restored VCP 0x10 values, original and restored internal brightness, selected
bus/selector, daemon/IPC results, and relevant journal output.
