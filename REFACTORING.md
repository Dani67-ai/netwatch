# Refactoring Roadmap

Known opportunities to improve readability and maintainability. Contributions welcome — pick an item, open an issue to claim it, and reference this file in the PR.

Items are grouped by impact (high → low). None of these change runtime behaviour.

---

## High impact

### 1. Split `app.rs` event loop into focused handlers

**Where:** `src/app.rs` lines ~723–1492  
**Problem:** The keyboard event match arm is ~770 lines with deeply nested conditionals for filter input, settings, tab-specific controls, and global shortcuts. It's the single hardest place for new contributors to navigate.  
**Fix:** Extract into focused functions called from the main match:

```rust
fn handle_settings_keys(app: &mut App, key: KeyEvent) -> bool { ... }
fn handle_filter_input(app: &mut App, key: KeyEvent) -> bool { ... }
fn handle_tab_keys(app: &mut App, key: KeyEvent) -> bool { ... }
fn handle_global_keys(app: &mut App, key: KeyEvent) { ... }
```

Each returns `bool` indicating whether the event was consumed. The main match becomes ~30 lines.

---

### 2. Name TCP flag constants

**Where:** `src/collectors/packets.rs`, `src/app.rs` (feed_network_intel)  
**Problem:** `flags & 0x02 != 0` appears throughout with no context. Maintainers need external references to understand protocol logic.  
**Fix:** Define in `packets.rs` and use everywhere:

```rust
pub const TCP_FLAG_FIN: u8 = 0x01;
pub const TCP_FLAG_SYN: u8 = 0x02;
pub const TCP_FLAG_RST: u8 = 0x04;
pub const TCP_FLAG_ACK: u8 = 0x10;
pub const TCP_FLAG_URG: u8 = 0x20;
```

---

### 3. Enumerate settings cursor positions

**Where:** `src/ui/settings.rs`, `src/app.rs`  
**Problem:** Settings are referenced by magic index throughout (e.g., `app.settings_cursor == 12` means AI Insights). `SETTINGS_COUNT: usize = 15` has no relationship to the actual rows.  
**Fix:**

```rust
#[repr(usize)]
pub enum SettingsField {
    Theme           = 0,
    DefaultTab      = 1,
    RefreshRate     = 2,
    // ...
    AiInsights      = 12,
    AiModel         = 13,
    AiEndpoint      = 14,
}
pub const SETTINGS_COUNT: usize = 15; // must equal variant count
```

Use `SettingsField::AiInsights as usize` instead of `12` everywhere.

---

### 4. Extract a `render_frame` layout helper

**Where:** Every file in `src/ui/`  
**Problem:** All 9 tab modules repeat the same 3-chunk vertical layout (header/content/footer). Any layout change requires editing 9 files.  
**Fix:** Add to `src/ui/widgets.rs`:

```rust
pub struct FrameChunks {
    pub header: Rect,
    pub content: Rect,
    pub footer: Rect,
}

pub fn frame_layout(area: Rect) -> FrameChunks {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);
    FrameChunks { header: chunks[0], content: chunks[1], footer: chunks[2] }
}
```

---

---

## Medium impact

### 7. Document the `analysis_loop` / `DnsCache` state machines

**Where:** `src/collectors/insights.rs`, `src/collectors/packets.rs`  
**Problem:** `DnsCache` has a `Pending` state with no timeout — if the resolution thread stalls, lookups return `None` forever. `InsightsStatus` transitions aren't documented.  
**Fix:** Add a timestamp to `Pending` entries and expire after 5 seconds. Add doc comments to both state machines describing transitions.

---

### 8. Deduplicate scroll handling

**Where:** `src/app.rs` lines ~1350–1693  
**Problem:** Identical scroll delta logic (`saturating_sub(1)`, `+ 3`, `.min(max)`) is repeated across Up/Down keys and mouse scroll for each tab.  
**Fix:** A helper closure or function:

```rust
fn clamp_scroll(current: usize, delta: isize, max: usize) -> usize {
    ((current as isize + delta).max(0) as usize).min(max)
}
```

---

### 9. Annotate packet capture constants

**Where:** `src/collectors/packets.rs` lines 8–14  
**Problem:** `MAX_PACKETS`, `MAX_STREAM_SEGMENTS`, `CAPTURE_SNAPLEN` have no comments explaining why they are set as they are.  
**Fix:** Add inline comments explaining the rationale and tradeoffs.

---

### 10. Add `NetwatchConfig::validate()`

**Where:** `src/config.rs`  
**Problem:** `refresh_rate_ms` is clamped in `app.rs` after loading — validation is separated from the type that owns the data. Invalid configs are silently corrected at use-site.  
**Fix:**

```rust
impl NetwatchConfig {
    pub fn validate(&mut self) {
        self.refresh_rate_ms = self.refresh_rate_ms.clamp(100, 5000);
        if self.theme.is_empty() { self.theme = "dark".into(); }
        // etc.
    }
}
```

Call in `load()` and expose for tests.

---

## Lower impact / cleanup

### 11. Tighten Tokio feature flags

**Where:** `Cargo.toml`  
**Problem:** `tokio = { features = ["full"] }` pulls in every Tokio subsystem. Only a subset is used.  
**Fix:** Replace with explicit features: `["rt-multi-thread", "sync", "time"]`.

### 12. Remove stale dead code

**Where:** Various  
**Problem:** `cargo clippy` surfaces `#[allow(dead_code)]` annotations and unused items. These accumulate as features evolve.  
**Fix:** Audit and either delete unused code or document why it exists.

### 13. Document the tick/render lifecycle at the top of `run()`

**Where:** `src/app.rs`  
**Problem:** The relationship between tick timing, event handling, and rendering is implicit. New contributors reading the loop can't easily tell what order things happen.  
**Fix:** A 5-line comment block at the entry of `run()` explaining the design.

---

## Done (archived)

- **#5 Group App scroll fields** — `UiScrollState` struct extracted from `App`; 11 scroll/selection fields moved into `app.scroll.*`; `App` field count reduced from 60 to 50.
- **#6 Extract protocol parsers** — `UdpProtocolParser` and `TcpProtocolParser` traits defined; DNS, DHCP, SSDP, NTP, QUIC (UDP) and DNS-over-TCP, TLS, HTTP (TCP) implemented as unit structs; registered in `UDP_PARSERS` / `TCP_PARSERS` (`LazyLock`); if/else chains in `parse_udp` and `parse_tcp` replaced with trait dispatch loops.
- **#4 render_frame helper** — `FrameChunks` + `frame_layout()` added to `widgets.rs`; used by Connections and Processes.
- **#7 State machine docs + DnsCache timeout** — `DnsEntry::Pending` now carries `queued_at: Instant`; entries retry after 5s. State-transition doc comments added to `DnsEntry` and `InsightsStatus`.
- **#8 Scroll deduplication** — `clamp_scroll()` and `scroll_tab()` added; all Up/Down key arms and both mouse scroll arms replaced; ~120 lines of duplicated match removed.
- **#11 Tokio features** — narrowed from `"full"` to `["rt-multi-thread", "macros", "sync"]`.
- **#12 Dead code / clippy** — 33 auto-fixes applied across 15 files (lifetime elision, `map_or`, useless `format!`, auto-deref, `RangeInclusive::contains`, `&mut Vec` → `&mut [T]`, etc.).
- **#13 Lifecycle comment** — 6-line event loop design comment added at top of `run()`.

## Previously done

- **#1 Split event loop** — `AppEvent::Key` arm reduced from 775 lines to 5. Handlers extracted: `handle_key`, `handle_help_key`, `handle_settings_key`, `handle_filter_input`, `handle_bpf_input`, `handle_main_key`. Also extracted `sort_connections()` and `top_remote_ips()` which were duplicated 3× inline.
- **#2 TCP flag constants** — `TCP_FLAG_SYN/ACK/RST/FIN/PSH/URG` defined in `packets.rs`; all hex literals replaced.
- **#3 Settings cursor enum** — `settings::cursor` module with named constants (`THEME`, `AI_INSIGHTS`, etc.); magic integers eliminated from app.rs and settings.rs.
- **#9 Annotate packet capture constants** — inline comments added to `MAX_PACKETS`, `CAPTURE_SNAPLEN`, etc.
- **#10 NetwatchConfig::validate()** — added; called from `load()`; clamps refresh rate, fills empty string defaults.
