# Vendored Upstream Skin Assets

These files are kept outside `package/` solely so `tools/generate_skin_assets.py` can deterministically composite real upstream assets into `package/contents/ui/skins/*/assets/` without network access. `CMakeLists.txt` installs only `package/`, so none of these raw vendor files are shipped in the plasmoid package.

## `xbox360/`

1. **`Blade04208/xbox360-blades`** (`main` branch, [github.com/Blade04208/xbox360-blades](https://github.com/Blade04208/xbox360-blades))
   - `notifications/xenonLogo.png` — 3D Xenon Xbox 360 sphere
   - `notifications/notify_bg.png` — 4-quadrant Ring of Light housing
   - `notifications/notify_Indicator1.png` — glowing Ring of Light quadrant LED arc
   - `blades/blade_{1..4}_jewel.svg`, `blades/blade_{1..4}_bgcolor.svg` — curved blade jewel spines & dashboard tints
   - `blades/blade_grey_left.svg`, `blades/blade_grey_rt.svg`, `blades/wing-rim.svg`, `blades/wing-mask.svg` — brushed-metal side blades & bezel rim
   - `guide/glass.png`, `guide/header.svg`, `guide/footer.svg`, `guide/rail.svg` — Guide brushed-metal texture & chrome header/footer/rail

2. **`gilbert-p/xbox_dashboard`** (`master` branch, [github.com/gilbert-p/xbox_dashboard](https://github.com/gilbert-p/xbox_dashboard))
   - `guideline_background.png`, `carbon_guideline_background.png` — Light Pearl-Silver & Carbon Dark Guide backdrops
   - `background_energy_spin.svg`, `power_rings_img.png` — Xbox 360 concentric energy rings
   - `guide_menu_button.png`, `glass-overlay.png` — Guide chrome button bevel & glass sheen

## `nothing/`

1. **`jaxparrow07/nothing-kde-widgets`** (`main` branch, GPL-3.0, [github.com/jaxparrow07/nothing-kde-widgets](https://github.com/jaxparrow07/nothing-kde-widgets))
   - `sunny.svg`, `cloudy.svg`, `partly_cloudy_day.svg`, `partly_cloudy_night.svg`, `rain_or_mist.svg`, `snow_fall.svg`, `thunder.svg` — official Nothing OS circular dot-matrix weather icons on the 389×389 grid (`r = 17.1`, pitch `34.2`)

2. **`Avadhkumar-geek/nothing_weather_clone`** (`master` branch, [github.com/Avadhkumar-geek/nothing_weather_clone](https://github.com/Avadhkumar-geek/nothing_weather_clone))
   - `sunrise.svg`, `sunset.svg` — visual reference for Nothing Weather's parabolic solar horizon glyphs
