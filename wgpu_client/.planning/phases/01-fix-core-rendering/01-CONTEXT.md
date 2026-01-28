# Phase 1: Fix Core Rendering - Context

**Gathered:** 2026-01-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix critical rendering bugs in the GPU-accelerated IDE: disappearing sidebar elements, invisible tab text, layout overflow into tab bar, and hit detection issues. All UI elements must render correctly with no progressive corruption.

</domain>

<decisions>
## Implementation Decisions

### Visual polish level
- Fix bugs AND polish touched components while modifying them
- Full visual polish: consistent spacing/alignment, clean visual hierarchy, modern aesthetics
- Style reference: Zed/Linear — minimal, modern, whitespace, subtle animations
- Dark mode only (light mode can come in a future phase)

### Debug tooling
- Debug overlays enabled via compile-time flag (#[cfg(debug_assertions)])
- Full debug visualization: component bounds, render order/z-index, hit detection regions
- Strip debug tooling completely in release builds (zero overhead)
- Console/log output: Claude's discretion based on what's useful for specific bugs

### Failure handling
- Component render failures: Claude decides per failure type (error placeholder vs log-and-skip vs crash)
- GPU/shader failures: Fall back to CPU rendering (software renderer as backup)
- Error logging: Both file and stderr
- Log verbosity: Configurable via env var (e.g., MUDA_LOG=debug)

### State during fixes
- Resetting UI state (selected tab, sidebar item) is acceptable if needed for fixes
- Scroll position in text areas: Must preserve (critical for user experience)
- Session state: Remember and restore last session across app restarts
- State persistence: Config file (JSON or TOML) in app data directory

### Claude's Discretion
- Exact debug overlay colors and visual treatment
- Whether to log to console vs file for specific error types
- Per-component decision on crash vs placeholder vs silent skip
- Config file format choice (JSON vs TOML)

</decisions>

<specifics>
## Specific Ideas

- Visual style should feel like Zed or Linear — clean, minimal, lots of whitespace, subtle animations
- Dark mode focus for now; don't invest in light theme yet

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-fix-core-rendering*
*Context gathered: 2026-01-28*
