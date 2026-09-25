# How to Revert README and Preview Images to Originals

This directory contains pristine backups of the original `README.md` and the 5 original preview screenshots before the September 2026 update.

## Revert Commands (One-Liner)

```bash
# From repository root:
cp backups/readme/README.original.md README.md
cp backups/readme/images/SunReactor_*.png docs/images/
git add README.md docs/images/
git commit -m "revert: restore original README and preview screenshots from backups/readme"
git push origin main
```

## Inventory of Backed-Up Assets
- `README.original.md`: Original README from refactor baseline (`c1c9bdc`).
- `README.main-v0.11.md`: Original README from GitHub `origin/main` (`d30273e`).
- `images/SunReactor_1.png`: Original dashboard screenshot.
- `images/SunReactor_2.png`: Original monitor focus screenshot.
- `images/SunReactor_3.png`: Original theme menu screenshot.
- `images/SunReactor_4.png`: Original weather chart screenshot.
- `images/SunReactor_5.png`: Original settings screenshot.
