# VDraw Industrial Laser & CNC Machine Compatibility Specification

*Post-Directive Architecture Overhaul*

---

## 1. Supported Hardware Dialects & Protocols

VDraw separates machine-independent CAM toolpaths from machine-specific postprocessors (`crates/core-cam/src/postprocessor.rs`).

| Controller Family | Standard Dialect | Laser Mode Specifics | Max Spindle / PWM | Air Assist |
| :--- | :--- | :--- | :---: | :---: |
| **GRBL 1.1f / 1.1h** | RS-274 / GRBL | `$32=1` Laser Mode, `M4` Dynamic Laser Power, `$H` Homing | `0..1000` (`S1000`) | `M8` / `M9` |
| **Smoothieboard** | Smoothieware G-code | Linear laser moves with `M3` / `M5` | `0..1.0` or `0..1000` | Supported |
| **Marlin 2.0+** | Marlin Laser | `M3 S...` spindle/laser enable, `M5` disable | `0..255` (`S255`) | `M8` / `M9` |
| **LinuxCNC / Mach3 / Mach4** | Standard RS-274 | Standard G-code (`G21`, `G90`, `M3 S...`, `G1`, `G0`, `M2`) | `0..1000` | Supported |
| **Vinyl / Pen Plotters** | HPGL / PLT | HPGL-2 standard (`IN; SP1; PU...; PD...; SP0;`) | 40 units/mm (1016 DPI) | N/A |
| **Ruida DSP Controllers** | Standard 16-color palette | RUIDA palette color mapping (`RUIDA_PALETTE[0..16]`) | Controller configured | Automatic |

---

## 2. Safety Invariants & Simulated Controller (`core-cam/src/safety.rs`)

VDraw enforces strict manufacturing safety invariants prior to exporting any job:

1. **Laser-Off During Rapids:**
   - The laser is strictly powered off (`M5` or `S0`) during all rapid travel moves ($G0$).
   - A simulated controller walks the entire toolpath sequence to verify that laser firing coincides only with cutting feed moves ($G1$).

2. **Soft-Limit Bed Verification:**
   - All toolpath coordinates and rapid travel endpoints are verified against physical bed boundaries:
     $$\mathbf{P} \in [0.0, W_{\text{bed}}] \times [0.0, H_{\text{bed}}]$$
   - Out-of-bounds moves trigger a `SafetyIssue::SoftLimitExceeded` error, preventing machine head crashes.

3. **Feed-Rate Enforcement:**
   - Commanded speeds are clamped to `max_feed_speed_mm_s` (default $1,000\,\text{mm/s}$) and `max_rapid_speed_mm_s` (default $2,000\,\text{mm/s}$).

4. **Finite Coordinate Guarantee:**
   - Any `NaN`, `Inf`, or uninitialized floating-point coordinate produces a `SafetyIssue::InvalidCoordinate`, immediately aborting postprocessing.

---

## 3. Kinematic Profiling & Time Estimation (`core-cam/src/estimator.rs`)

To replace unrealistic constant-speed estimates, VDraw models physical CNC trapezoidal velocity profiles:

- **Trapezoidal Motion Modeling:**
  For any move of distance $L$, target velocity $V_{\text{target}}$, and machine acceleration $A$:
  - Critical ramp distance:
    $$d_{\text{ramp}} = \frac{V_{\text{target}}^2}{A}$$
  - If $L \ge d_{\text{ramp}}$ (cruising move):
    $$t = 2 \frac{V_{\text{target}}}{A} + \frac{L - d_{\text{ramp}}}{V_{\text{target}}}$$
  - If $L < d_{\text{ramp}}$ (short move, peak velocity $V_{\text{peak}} = \sqrt{L \cdot A}$):
    $$t = 2 \sqrt{\frac{L}{A}}$$

- **Pierce Dwell Delay:**
  Configurable pierce time allowance (default $0.05\,\text{s}$ per contour) models beam penetration through thick acrylic or plywood before motion begins.

- **Telemetry Transparency:**
  Every estimate struct (`CamJobEstimation`) exposes `is_controller_verified: false` to clearly communicate that duration estimates are kinematic models rather than hardware lookahead feedback.
