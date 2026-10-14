#version 440

/*
 * The XMB wave of the PlayStation 3 skin: a full-screen procedural background
 * drawn by skins/ps3/Waves.qml. Built into xmb.frag.qsb by tools/build_shaders.sh
 * (Qt's qsb); the .qsb is committed, so the widget needs no build tools.
 *
 * Time is a loop of PERIOD seconds (Waves.qml's period must match). Every
 * harmonic completes a whole number of cycles across the width (m) and of loops
 * in time (n), so the loop closes without a seam; a harmonic with m cycles and
 * n loops travels right at n / m widths per loop. The ribbons are summed
 * additively, so where two cross they brighten, as silk does.
 */

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    float time;         // seconds into the loop, 0 to PERIOD
    vec2 resolution;    // the item's size in logical px
    vec4 colour;        // the ribbons' tint (palette accent, or the month's)
    vec4 sheen;         // the white-blue highlight at the cores
    vec4 deep;          // the backdrop's top colour
    float nightFactor;  // 0 by day, 1 at night: the whole picture dims
};

const float PERIOD = 40.0;
const float TAU = 6.283185307;
/* The bottom of the backdrop: near-black navy. */
const vec3 NAVY = vec3(0.008, 0.016, 0.046);

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

/* The offset of a ribbon's centre line at x (0 to 1 across the popup), in units
   of its amplitude. Each harmonic is h = (m, n, phase, weight). */
float wave(float x, float tau, vec4 h1, vec4 h2, vec4 h3) {
    return h1.w * sin(TAU * (h1.x * x - h1.y * tau) + h1.z)
         + h2.w * sin(TAU * (h2.x * x - h2.y * tau) + h2.z)
         + h3.w * sin(TAU * (h3.x * x - h3.y * tau) + h3.z);
}

/* One ribbon, added to col: a bright core of half-width thick, inside a wider
   translucent halo (2.6 times as wide, a third as bright), about its centre line.
   The crest (its phase) runs along it: brighter where it passes, fading in the
   tails between crests. Multiplies, not pow(): this runs for every pixel. */
void ribbon(vec2 uv, float tau, float y0, float amp, float thick, float strength,
            vec4 h1, vec4 h2, vec4 h3, float crest, inout vec3 col) {
    float yc = y0 + amp * wave(uv.x, tau, h1, h2, h3);
    float d = uv.y - yc;
    float c1 = d / thick;
    float c2 = d / (thick * 2.6);
    float core = exp(-2.5 * c1 * c1);
    float halo = exp(-1.4 * c2 * c2);
    float u = 0.5 + 0.5 * cos(TAU * (uv.x - tau) + crest);
    float along = 0.12 + 0.88 * u * u * u;
    col += colour.rgb * (core + 0.55 * halo) * along * strength;
    float core2 = core * core;
    col += sheen.rgb * (core2 * core2 * core2) * along * strength * 0.6;
}

/* A triangle wave, 0 to 1 with period 1: the cheap stand-in for a sine in the loop. */
float tri(float x) {
    return abs(fract(x) * 2.0 - 1.0);
}

/* Faint drifting specks: one candidate per cell, lit for about half of them.
   Each drifts in a small loop within its cell (kept clear of the edges, so a
   pixel needs only its own cell) and twinkles; both are whole loops in time. */
float motes(vec2 px, float cell, float seed, float tau) {
    vec2 c = floor(px / cell);
    if (hash12(c + seed) > 0.55) {
        return 0.0;
    }
    vec2 h = hash22(c + seed);
    vec2 drift = vec2(tri(tau + h.x), tri(tau + h.y)) - 0.5;
    vec2 at = (c + 0.3 + 0.4 * h + 0.3 * drift) * cell;
    float d = length(px - at);
    float twinkle = 0.45 + 0.55 * tri(3.0 * tau + 5.0 * h.y);
    return exp(-d * d / 1.6) * twinkle;
}

void main() {
    vec2 uv = qt_TexCoord0;         // (0, 0) at the top left
    float tau = time / PERIOD;
    float asp = resolution.x / max(resolution.y, 1.0);

    // The backdrop: deep blue at the top, near-black navy at the bottom, with a
    // faint lift through the middle where the content sits.
    vec3 col = mix(deep.rgb, NAVY, smoothstep(0.0, 1.0, uv.y));
    float horizon = (uv.y - 0.6) / 0.3;
    col += colour.rgb * 0.035 * exp(-horizon * horizon);

    // Three strong ribbons, widest tails first; each has a main drift (m = 1,
    // n = 1) and two slower harmonics that undulate it.
    ribbon(uv, tau, 0.34, 0.095, 0.026, 0.25,
           vec4(1.0, 1.0, 0.0, 0.66), vec4(2.0, 1.0, 1.9, 0.20), vec4(3.0, 2.0, 0.7, 0.07), 0.0, col);
    ribbon(uv, tau, 0.56, 0.110, 0.024, 0.27,
           vec4(1.0, 1.0, 2.4, 0.66), vec4(2.0, 3.0, 0.5, 0.20), vec4(3.0, 1.0, 3.6, 0.06), 2.0, col);
    ribbon(uv, tau, 0.76, 0.085, 0.025, 0.23,
           vec4(1.0, 1.0, 4.2, 0.66), vec4(2.0, 2.0, 1.1, 0.20), vec4(3.0, 3.0, 0.2, 0.06), 4.0, col);

    // Three faint, broad ribbons between them.
    ribbon(uv, tau, 0.16, 0.060, 0.050, 0.080,
           vec4(1.0, 1.0, 1.0, 0.70), vec4(2.0, 2.0, 3.0, 0.30), vec4(3.0, 1.0, 0.5, 0.0), 1.0, col);
    ribbon(uv, tau, 0.46, 0.070, 0.055, 0.080,
           vec4(1.0, 1.0, 5.1, 0.70), vec4(2.0, 1.0, 0.8, 0.30), vec4(3.0, 2.0, 2.2, 0.0), 3.0, col);
    ribbon(uv, tau, 0.90, 0.060, 0.050, 0.080,
           vec4(1.0, 1.0, 2.9, 0.70), vec4(2.0, 3.0, 4.4, 0.30), vec4(3.0, 1.0, 1.7, 0.0), 5.0, col);

    // Dust: two layers, the larger ones fainter and slower to read.
    vec2 px = uv * resolution;
    col += sheen.rgb * (0.22 * motes(px, 46.0, 11.0, tau) + 0.10 * motes(px, 17.0, 29.0, tau));

    // A subtle vignette, and the night dims everything by about a third.
    vec2 q = (uv - vec2(0.5, 0.4)) * vec2(asp, 1.0);
    col *= 1.0 - 0.32 * smoothstep(0.4, 1.0, length(q));
    col *= 1.0 - 0.32 * nightFactor;

    fragColor = vec4(clamp(col, 0.0, 1.0) * qt_Opacity, qt_Opacity);
}
