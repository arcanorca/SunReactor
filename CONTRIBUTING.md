# Contributing to SunReactor

First off, thank you for your interest and for taking the time to check out the project!

I'll be honest upfront: my software engineering knowledge is pretty minimal. When building this daemon, my main focus was on stability and keeping things simple. I am very open to any feedback, fixes, and architectural suggestions.

## How You Can Contribute

*   **Bug Reports:** If something breaks (especially regarding `ddcutil` or specific multi-monitor quirks), please open an issue. It helps a ton if you include your system details (distro, DE, monitor models).
*   **Feature Requests:** Open an issue and let's discuss it. The core philosophy of this daemon is to remain lightweight and stable, so I might not merge every single feature, but it never hurts to talk about it.
*   **Pull Requests:** 
    *   Please open an issue to discuss your proposed changes before submitting a PR. This just makes sure we aren't duplicating work.
    *   If possible, run `cargo fmt` and `cargo clippy` before pushing your code.
    *   Any PRs that change core architectural decisions (like trying to introduce an async runtime or changing the offline-first logic) require a solid discussion first.

## Setting up the Dev Environment

Cloning and building the project locally is straightforward:

```bash
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor
cargo build
```
