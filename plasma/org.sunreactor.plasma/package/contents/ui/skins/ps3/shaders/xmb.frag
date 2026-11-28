#version 440

/*
 * The XMB Silk & Glitter background of the PlayStation 3 skin (Firmware 3.00+):
 * a translucent 3D parametric silk curtain integrated across depth slices with
 * Fresnel caustic fold creases, a secondary deeper veil, and smoothly gliding
 * glitter sparkles + soft bokeh orbs drifting diagonally along the wave.
 *
 * Time is a seamless loop of PERIOD seconds (Waves.qml's period must match):
 * wave harmonics complete integer cycles across PERIOD, and glitter/bokeh grids
 * shift by exact integer multiples of their wrap period.
 */

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    float time;         // seconds into the loop, 0 to PERIOD
    vec2 resolution;    // the item's size in logical px
    vec4 colour;        // the silk's tint (palette accent, or the month's)
    vec4 sheen;         // the icy-white/cyan specular caustic tint
    vec4 deep;          // the backdrop's luminous upper royal-sapphire colour
    float nightFactor;  // 0 by day, 1 at night: deepens mood without muddying contrast
};

const float PERIOD = 24.0;
const float TAU = 6.283185307;
const vec3 NAVY = vec3(0.006, 0.015, 0.048);

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec2 hash22(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

/* Parametric 3D height of one silk curtain at screen x and depth z in [0, 1].
   Notice the gentle diagonal PS3 XMB tilt (+upward left->right) and counter-
   travelling harmonics across depth so fold creases split, cross, and roll. */
float silkY(float x, float z, float tau, float yCenter, float tilt, float amp, float seed) {
    float ph = TAU * tau;
    float xRot = x + z * 0.32;
    float swell = 0.62 * sin(TAU * (1.15 * x - 1.0 * tau) + z * 3.1 + seed)
                + 0.34 * sin(TAU * (2.10 * x + 1.0 * tau) - z * 4.4 + seed * 1.7)
                + 0.18 * sin(TAU * (3.20 * xRot - 2.0 * tau) + z * 6.2 + seed * 2.3)
                + 0.14 * cos(ph + z * 4.8 + seed * 0.7);
    return yCenter + tilt * (0.5 - x) + amp * swell;
}

/* Integrate N constant depth slices through a 3D silk sheet.
   Where neighbouring slices converge (foldSlope -> 0), grazing incidence
   produces crisp luminous Fresnel caustics along the fold creases. */
void accumulateSilk(vec2 uv, float tau, float yCenter, float tilt, float amp,
                    float thick, float seed, float baseStrength, float causticStrength,
                    out vec3 silkCol) {
    silkCol = vec3(0.0);
    const int SLICES = 22;
    float dz = 1.0 / float(SLICES - 1);
    float yPrev = silkY(uv.x, 0.0, tau, yCenter, tilt, amp, seed);

    for (int i = 0; i < SLICES; ++i) {
        float z = float(i) * dz;
        float yc = (i == 0) ? yPrev : silkY(uv.x, z, tau, yCenter, tilt, amp, seed);
        float foldSlope = abs(yc - yPrev) / dz;
        yPrev = yc;

        float d = uv.y - yc;
        float cBody = d / thick;
        float cCore = d / (thick * 0.38);

        float veil = exp(-1.35 * cBody * cBody);
        float crease = exp(-2.8 * cCore * cCore);

        // Grazing-angle caustic spike where the folded sheet goes tangent to view rays
        float caustic = clamp(0.11 / (0.045 + foldSlope * foldSlope * 1.8), 0.0, 2.2);
        // Subtle front/back hem accent
        float hem = (i == 0 || i == SLICES - 1) ? 1.55 : 1.0;
        // Fade gracefully toward rear depth slices
        float depthFade = 0.55 + 0.45 * (1.0 - z);

        float along = 0.58 + 0.42 * cos(TAU * (0.85 * uv.x - tau) + z * 2.4 + seed);
        float w = along * depthFade * dz;

        silkCol += colour.rgb * veil * baseStrength * w;
        silkCol += mix(colour.rgb, sheen.rgb, 0.78) * crease * caustic * causticStrength * hem * w;
    }
}

/* Lighter background veil pass (12 slices) for layered silk depth. */
void accumulateVeil(vec2 uv, float tau, float yCenter, float tilt, float amp,
                    float thick, float seed, float baseStrength, float causticStrength,
                    inout vec3 silkCol) {
    const int VEIL_SLICES = 12;
    float dz = 1.0 / float(VEIL_SLICES - 1);
    float yPrev = silkY(uv.x, 0.0, tau, yCenter, tilt, amp, seed);

    for (int i = 0; i < VEIL_SLICES; ++i) {
        float z = float(i) * dz;
        float yc = (i == 0) ? yPrev : silkY(uv.x, z, tau, yCenter, tilt, amp, seed);
        float foldSlope = abs(yc - yPrev) / dz;
        yPrev = yc;

        float d = uv.y - yc;
        float cBody = d / thick;
        float cCore = d / (thick * 0.44);
        float veil = exp(-1.2 * cBody * cBody);
        float crease = exp(-2.4 * cCore * cCore);
        float caustic = clamp(0.09 / (0.055 + foldSlope * foldSlope * 2.1), 0.0, 1.7);
        float depthFade = 0.45 + 0.55 * (1.0 - z);

        float w = depthFade * dz;
        silkCol += colour.rgb * veil * baseStrength * w;
        silkCol += sheen.rgb * crease * caustic * causticStrength * w;
    }
}

/* Seamless diagonally drifting glitter sparkles (exact periodic wrap). */
float glitterField(vec2 px, float cell, vec2 wrapCells, vec2 driftCells, float seed, float tau, float orbMode) {
    vec2 shifted = px - driftCells * cell * tau;
    vec2 cellIdx = floor(shifted / cell);
    vec2 wrapped = mod(cellIdx, wrapCells);

    if (hash12(wrapped + seed) > (orbMode > 0.5 ? 0.36 : 0.48)) {
        return 0.0;
    }

    vec2 h = hash22(wrapped + seed + 7.13);
    vec2 localPos = (cellIdx + 0.22 + 0.56 * h) * cell;
    vec2 delta = shifted - localPos;
    float r = length(delta);

    float shimmer = 0.25 + 0.75 * pow(0.5 + 0.5 * sin(TAU * (2.0 * tau + h.x) + h.y * TAU), 2.5);
    if (orbMode > 0.5) {
        // Soft defocused PS3 bokeh orb
        float rad = mix(2.2, 4.4, h.y);
        return smoothstep(rad, rad * 0.25, r) * 0.38 * shimmer;
    }

    // Sharp pinpoint sparkle + subtle 4-point cross flare on brighter motes
    float core = exp(-r * r / 1.15);
    float starH = exp(-abs(delta.y) * 2.2) * exp(-abs(delta.x) * 0.38);
    float starV = exp(-abs(delta.x) * 2.2) * exp(-abs(delta.y) * 0.38);
    float crossFlare = (h.x > 0.65) ? 0.45 * (starH + starV) : 0.0;
    return (core + crossFlare) * shimmer;
}

void main() {
    vec2 uv = qt_TexCoord0; // (0, 0) top-left
    float tau = time / PERIOD;
    float asp = resolution.x / max(resolution.y, 1.0);

    // Rich PS3 XMB sapphire gradient: bright royal band mid-upper, midnight navy bottom
    vec3 skyTop = mix(deep.rgb, NAVY, 0.28);
    vec3 skyMid = mix(colour.rgb * 0.52, deep.rgb, 0.45);
    vec3 col = mix(skyTop, skyMid, smoothstep(0.0, 0.44, uv.y));
    col = mix(col, NAVY, smoothstep(0.42, 1.0, uv.y));

    // Soft atmospheric studio bloom behind the silk sheet
    float bandDiff = (uv.y - (0.54 + 0.08 * (0.5 - uv.x))) / 0.30;
    col += colour.rgb * 0.14 * exp(-bandDiff * bandDiff);

    // Background counter-rolling chiffon veil + primary caustic silk curtain
    vec3 silk = vec3(0.0);
    accumulateSilk(uv, tau, 0.57, 0.095, 0.095, 0.042, 0.40, 0.92, 0.78, silk);
    accumulateVeil(uv, tau, 0.44, -0.055, 0.075, 0.058, 2.70, 0.42, 0.34, silk);

    // Softly roll off overbright caustic overlaps so white XMB text stays crisp
    silk = silk / (vec3(1.0) + silk * 0.55);
    col += silk;

    // Firmware 3.00+ floating glitter sparkles + soft bokeh confined mainly to the wave band
    float waveBand = 0.20 + 0.80 * exp(-bandDiff * bandDiff * 0.85);
    vec2 px = uv * resolution;
    float sparkles = glitterField(px, 34.0, vec2(12.0, 6.0), vec2(12.0, -6.0), 13.0, tau, 0.0);
    float bokeh = glitterField(px, 68.0, vec2(8.0, 4.0), vec2(8.0, -4.0), 41.0, tau, 1.0);
    vec3 glitterTint = mix(sheen.rgb, vec3(1.0), 0.45);
    col += glitterTint * (0.58 * sparkles + 0.34 * bokeh) * waveBand;

    // Gentle edge vignette + subtle night mood (keeps XMB silk luminous at night)
    vec2 q = (uv - vec2(0.5, 0.44)) * vec2(asp, 1.0);
    col *= 1.0 - 0.24 * smoothstep(0.45, 1.05, length(q));
    col = mix(col, col * vec3(0.76, 0.82, 0.94), 0.75 * nightFactor);

    fragColor = vec4(clamp(col, 0.0, 1.0) * qt_Opacity, qt_Opacity);
}
