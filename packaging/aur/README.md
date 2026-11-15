# AUR packaging for SunReactor

Two split-package PKGBUILDs, each producing a daemon/CLI package and a Plasma 6 widget package:

| Directory | pkgbase | Packages | Source |
|---|---|---|---|
| `sunreactor/` | `sunreactor` | `sunreactor`, `plasma6-applets-sunreactor` | GitHub tag `v$pkgver` tarball |
| `sunreactor-git/` | `sunreactor-git` | `sunreactor-git`, `plasma6-applets-sunreactor-git` | `git+https://github.com/arcanorca/SunReactor.git` |

Publishing to the AUR is a **manual step by the maintainer**. Nothing in this repository pushes to aur.archlinux.org.

## Test locally

```sh
cd packaging/aur/sunreactor-git      # or sunreactor/ (needs the v$pkgver tag to exist on GitHub)
makepkg -f            # build + run cargo/ctest checks; add -s to install makedepends
namcap PKGBUILD
namcap *.pkg.tar.zst
makepkg -si           # build and install (uses sudo)
```

To test unpublished work, copy the PKGBUILD to a scratch directory and change the source to
`"$pkgbase::git+file:///path/to/repo#branch=<branch>"`.

## Release / update

1. Tag the release `vX.Y.Z` (matching `Cargo.toml`) and push the tag on GitHub.
2. In `sunreactor/PKGBUILD`: set `pkgver=X.Y.Z`, `pkgrel=1`.
3. `updpkgsums` (replaces the `SKIP` placeholder with the tarball sha256).
4. `makepkg -f` and `namcap` to verify.
5. `makepkg --printsrcinfo > .SRCINFO`, in both directories (`sunreactor-git` only when its PKGBUILD changes; its `pkgver` is computed by `pkgver()`).
6. Copy `PKGBUILD`, `.SRCINFO`, `*.install` into the AUR git clone (`ssh://aur@aur.archlinux.org/sunreactor.git`), commit and push by hand.

## Notes

- `ddcutil` is an optdepend: the daemon also works with sysfs backlights (`brightnessctl`) and detects ddcutil at runtime. DDC/CI needs access to `/dev/i2c-*`; the `.install` message explains the `i2c-dev` module and `i2c` group.
- The systemd unit template in `contrib/systemd/` has installer placeholders; the PKGBUILD renders it for `/usr/bin` and drops the `XDG_*_HOME` overrides.
- The plasmoid installs to the standard KF6 locations (`/usr/share/plasma/plasmoids/org.sunreactor.plasma`, QML module in `/usr/lib/qt6/qml/org/sunreactor/plasma`), which are on Plasma's default import path, so the installer's `QML_IMPORT_PATH` environment files are not needed.
