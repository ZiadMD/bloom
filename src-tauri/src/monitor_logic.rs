//! Platform-free decisions behind multi-monitor support (see monitors.rs).
//!
//! Everything here works on plain numbers so it can be unit tested without
//! windows, webviews or a display: which monitor setting is active, how a
//! window label maps to a surface, the edge "pressure" gate, and when an extra
//! notch or dock should take clicks, report a hover, or step aside for another
//! window. The rules mirror the primary notch and dock in services.rs.

/// Which of Bloom's two surfaces a window shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surface {
    Notch,
    Dock,
}

impl Surface {
    /// Label of the surface's original window, the one on the primary (or
    /// chosen) monitor.
    pub fn base_label(self) -> &'static str {
        match self {
            Surface::Notch => "main",
            Surface::Dock => "dock",
        }
    }

    /// Settings key holding the monitor mode: "primary", "all" or "specific".
    pub fn mode_key(self) -> &'static str {
        match self {
            Surface::Notch => "bloom-notch-monitor-mode",
            Surface::Dock => "bloom-dock-monitor-mode",
        }
    }

    /// Settings key holding the chosen monitor's id in "specific" mode.
    pub fn id_key(self) -> &'static str {
        match self {
            Surface::Notch => "bloom-notch-monitor-id",
            Surface::Dock => "bloom-dock-monitor-id",
        }
    }

    /// Label of the extra window this surface gets on another monitor.
    pub fn extra_label(self, monitor_id: &str) -> String {
        format!("{}-{}", self.base_label(), monitor_id)
    }

    /// The surface an extra window label belongs to. The original windows
    /// ("main", "dock") are not extras.
    pub fn of_extra_label(label: &str) -> Option<Surface> {
        if label.strip_prefix("main-").is_some_and(|id| !id.is_empty()) {
            Some(Surface::Notch)
        } else if label.strip_prefix("dock-").is_some_and(|id| !id.is_empty()) {
            Some(Surface::Dock)
        } else {
            None
        }
    }
}

/// Where a surface is shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MonitorMode {
    /// Only on the primary monitor (Bloom's original behavior).
    Primary,
    /// On the primary monitor plus an extra window on every other monitor.
    All,
    /// Only on the monitor the user picked.
    Specific,
}

impl MonitorMode {
    /// Unknown or missing values fall back to the primary monitor, so a
    /// settings file from an older or newer Bloom never hides a surface.
    pub fn parse(value: Option<&str>) -> MonitorMode {
        match value {
            Some("all") => MonitorMode::All,
            Some("specific") => MonitorMode::Specific,
            _ => MonitorMode::Primary,
        }
    }
}

/// A stable, label-safe id for a monitor. Windows names monitors like
/// `\\.\DISPLAY2`; the id keeps the meaningful part (`DISPLAY2`). Window
/// labels only allow `a-zA-Z0-9-/:_`, so anything else becomes `_`. Without a
/// name, the monitor's position identifies it.
pub fn monitor_id(name: Option<&str>, x: i32, y: i32) -> String {
    let trimmed = name
        .map(|n| n.trim_start_matches(['\\', '.', '?']))
        .filter(|n| !n.is_empty());
    let raw = match trimmed {
        Some(n) => n.to_string(),
        None => format!("at{x}x{y}"),
    };
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// A friendly name for the settings list: "Display 2 (2560×1440)".
pub fn monitor_display_name(index: usize, id: &str, width: u32, height: u32) -> String {
    let digits: String = id
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let number = if id.starts_with("DISPLAY") && !digits.is_empty() {
        digits
    } else {
        (index + 1).to_string()
    };
    format!("Display {number} ({width}\u{d7}{height})")
}

/// Physical screen bounds, right/bottom exclusive like Win32 RECTs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bounds {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Bounds {
        Bounds {
            left: x,
            top: y,
            right: x + width as i32,
            bottom: y + height as i32,
        }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// A rectangle reported by a webview in its own CSS pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CssRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Height of the band at a screen edge that reveals a hidden surface.
pub const EDGE_BAND_PX: f64 = 8.0;

/// The hover-intent ("pressure") gate shared by the top and bottom edges.
/// Contact only starts a clock; the edge arms after the cursor rested in the
/// band for `dwell_ms`. Leaving the band disarms it. Mirrors
/// `top_edge_armed_for` / `dock_edge_armed_for` in services.rs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeGate {
    pub enter_ms: i64,
    pub armed: bool,
}

impl EdgeGate {
    pub fn reset(&mut self) {
        self.enter_ms = 0;
        self.armed = false;
    }

    pub fn step(&mut self, in_band: bool, now: i64, dwell_ms: i64) -> bool {
        if !in_band {
            self.reset();
            return false;
        }
        if self.armed {
            return true;
        }
        if dwell_ms <= 0 {
            self.armed = true;
            return true;
        }
        if self.enter_ms == 0 {
            self.enter_ms = now;
            return false;
        }
        if now - self.enter_ms >= dwell_ms {
            self.armed = true;
            return true;
        }
        false
    }
}

/// Whether `(x, y)` lies in the band along the top of `monitor`.
pub fn in_top_band(monitor: Bounds, x: i32, y: i32, scale: f64) -> bool {
    y <= monitor.top + (EDGE_BAND_PX * scale) as i32 && x >= monitor.left && x <= monitor.right
}

/// Whether `(x, y)` lies in the band along the bottom of `monitor`.
pub fn in_bottom_band(monitor: Bounds, x: i32, y: i32, scale: f64) -> bool {
    y >= monitor.bottom - (EDGE_BAND_PX * scale) as i32 && x >= monitor.left && x <= monitor.right
}

/// What an extra surface knows about itself between cursor updates.
#[derive(Clone, Copy, Debug)]
pub struct InteractionState {
    pub gate: EdgeGate,
    /// Until when a recent hover keeps the surface revealed.
    pub expiry_ms: i64,
    /// The webview says the pointer is over it.
    pub hovered: bool,
    /// Last applied click-through state: -1 unknown, 0 interactive, 1 ignore.
    pub last_ignore: i8,
    /// Last reported edge hover: -1 unknown, 0 or 1.
    pub last_edge_hover: i8,
}

impl Default for InteractionState {
    fn default() -> Self {
        InteractionState {
            gate: EdgeGate::default(),
            expiry_ms: 0,
            hovered: false,
            last_ignore: -1,
            last_edge_hover: -1,
        }
    }
}

/// What to do after an interaction update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InteractionEffects {
    /// Make the window click-through (true) or interactive (false).
    pub ignore: bool,
    /// The click-through state flipped: re-assert topmost.
    pub ignore_changed: bool,
    /// The edge hover changed to this value: tell the webview.
    pub edge_hover: Option<bool>,
    /// The window just became click-through while the webview thought the
    /// pointer was inside; it can no longer see the pointer leave.
    pub pointer_left: bool,
}

/// Shared tail of both surfaces: hover expiry, edge-hover reporting and the
/// click-through flip.
fn finish(
    state: &mut InteractionState,
    hovered: bool,
    interactive: bool,
    now: i64,
    clear_hover_on_ignore: bool,
) -> InteractionEffects {
    let mut effects = InteractionEffects::default();
    let final_hover = hovered || now < state.expiry_ms;
    let hover_val = i8::from(final_hover);
    if state.last_edge_hover != hover_val {
        state.last_edge_hover = hover_val;
        effects.edge_hover = Some(final_hover);
    }
    let ignore = !interactive;
    let ignore_val = i8::from(ignore);
    if state.last_ignore != ignore_val {
        state.last_ignore = ignore_val;
        effects.ignore_changed = true;
        if ignore && clear_hover_on_ignore && state.hovered {
            state.hovered = false;
            effects.pointer_left = true;
        }
    }
    effects.ignore = ignore;
    effects
}

/// Click-through everything and drop any hover: a fullscreen app on this
/// monitor or a capture tool owns the screen.
pub fn step_aside(state: &mut InteractionState, clear_hover: bool) -> InteractionEffects {
    state.gate.reset();
    state.expiry_ms = 0;
    let mut effects = InteractionEffects {
        ignore: true,
        ..Default::default()
    };
    if state.last_ignore != 1 {
        state.last_ignore = 1;
        effects.ignore_changed = true;
    }
    if state.last_edge_hover != 0 {
        state.last_edge_hover = 0;
        effects.edge_hover = Some(false);
    }
    if clear_hover && state.hovered {
        state.hovered = false;
        effects.pointer_left = true;
    }
    effects
}

/// Inputs for an extra notch's interaction update.
pub struct NotchInput {
    pub cursor: (i32, i32),
    pub now: i64,
    pub monitor: Bounds,
    /// The window's top-left corner on screen.
    pub window_origin: (i32, i32),
    pub scale: f64,
    /// The notch's box as reported by its webview.
    pub rect: Option<CssRect>,
    /// Fixed mode keeps the notch on screen, so the edge never reveals it.
    pub fixed_mode: bool,
    pub dwell_ms: i64,
    /// The notch is on screen (not hidden by smart/peek).
    pub visible: bool,
    /// The left button is held: a possible OS drag needs a drop target.
    pub drag_held: bool,
}

/// Hover and click-through for an extra notch; mirrors
/// `update_main_interaction` in services.rs.
pub fn notch_interaction(state: &mut InteractionState, input: &NotchInput) -> InteractionEffects {
    let (cx, cy) = input.cursor;
    let scale = input.scale;
    let in_band = in_top_band(input.monitor, cx, cy, scale);
    let edge_armed = if input.fixed_mode {
        state.gate.reset();
        false
    } else {
        state.gate.step(in_band, input.now, input.dwell_ms)
    };

    let hovered = edge_armed || state.hovered;
    if hovered {
        state.expiry_ms = input.now + 500;
    }

    let mut interactive = false;
    if let Some(r) = input.rect {
        let pad_x = (20.0 * scale) as i32;
        let pad_y_bottom = (5.0 * scale) as i32;
        let hyst = if state.last_ignore == 0 {
            (10.0 * scale) as i32
        } else {
            0
        };
        let (wx, wy) = input.window_origin;
        let rx = wx + (r.x as f64 * scale) as i32 - pad_x - hyst;
        let rw = (r.width as f64 * scale) as i32 + pad_x * 2 + hyst * 2;
        let ry_bottom = wy + (r.height as f64 * scale) as i32 + pad_y_bottom + hyst;
        let in_rect = cx >= rx && cx <= rx + rw && cy >= wy && cy <= ry_bottom;
        let edge_pad = (60.0 * scale) as i32;
        let in_top_span = in_band && cx >= rx - edge_pad && cx <= rx + rw + edge_pad;
        let hover_active = hovered || input.now < state.expiry_ms;
        if (input.visible || hover_active) && (in_rect || in_top_span) {
            interactive = true;
        }
        if input.drag_held && (in_rect || in_top_span) {
            interactive = true;
        }
    }

    finish(state, hovered, interactive, input.now, true)
}

/// Inputs for an extra dock's interaction update.
pub struct DockInput {
    pub cursor: (i32, i32),
    pub now: i64,
    pub monitor: Bounds,
    /// The dock window's bounds on screen.
    pub window: Bounds,
    pub scale: f64,
    /// The dock's hit area as reported by its webview.
    pub rect: Option<CssRect>,
    /// An open dock menu or popup, which takes clicks over itself.
    pub menu: Option<CssRect>,
    pub dwell_ms: i64,
}

/// Hover and click-through for an extra dock; mirrors
/// `update_dock_interaction` in services.rs.
pub fn dock_interaction(state: &mut InteractionState, input: &DockInput) -> InteractionEffects {
    let (cx, cy) = input.cursor;
    let scale = input.scale;
    let w = input.window;
    let mut interactive = false;
    let mut span: Option<(i32, i32)> = None;

    let in_window = cx >= w.left && cx <= w.right && cy >= w.top && cy <= w.bottom;
    if in_window {
        if let Some(r) = input.rect {
            let pad_x = (5.0 * scale) as i32;
            let pad_y_top = (8.0 * scale) as i32;
            let pad_y_bottom = (5.0 * scale) as i32;
            let hyst = if state.last_ignore == 0 {
                (10.0 * scale) as i32
            } else {
                0
            };
            let rx = w.left + (r.x as f64 * scale) as i32 - pad_x - hyst;
            let ry = w.top + (r.y as f64 * scale) as i32 - pad_y_top - hyst;
            let rw = (r.width as f64 * scale) as i32 + pad_x * 2 + hyst * 2;
            let rh = (r.height as f64 * scale) as i32 + pad_y_top + pad_y_bottom + hyst * 2;
            if cx >= rx && cx <= rx + rw && cy >= ry && cy <= ry + rh {
                interactive = true;
            }
            span = Some((rx, rx + rw));
        }
        if !interactive {
            if let Some(m) = input.menu {
                let pad = (5.0 * scale) as i32;
                let rx = w.left + (m.x as f64 * scale) as i32 - pad;
                let ry = w.top + (m.y as f64 * scale) as i32 - pad;
                let rw = (m.width as f64 * scale) as i32 + pad * 2;
                let rh = (m.height as f64 * scale) as i32 + pad * 2;
                if cx >= rx && cx <= rx + rw && cy >= ry && cy <= ry + rh {
                    interactive = true;
                }
            }
        }
    }

    let at_bottom = in_bottom_band(input.monitor, cx, cy, scale);
    let edge_armed = state.gate.step(at_bottom, input.now, input.dwell_ms);
    let hovered = edge_armed || state.hovered;
    if hovered {
        state.expiry_ms = input.now + 500;
    }
    if at_bottom && edge_armed {
        if let Some((left, right)) = span {
            let edge_pad = (60.0 * scale) as i32;
            if cx >= left - edge_pad && cx <= right + edge_pad {
                interactive = true;
            }
        }
    }

    finish(state, hovered, interactive, input.now, false)
}

/// The window on top of a monitor, as the overlap check sees it.
#[derive(Clone, Copy, Debug)]
pub struct TopWindow {
    /// Desktop, Start, Search or a taskbar: never hides Bloom.
    pub is_shell: bool,
    /// A tool window: transient, never hides Bloom.
    pub is_tool: bool,
    /// Outer bounds (DWM frame bounds when available).
    pub rect: Bounds,
    /// Client area in screen coordinates, if known.
    pub client: Option<Bounds>,
    pub maximized: bool,
    pub has_caption: bool,
}

/// How the top window on a monitor affects that monitor's surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OverlapResult {
    /// Smart mode should hide the surface.
    pub overlap: bool,
    /// A standard maximized window: the adaptive dock stretches.
    pub maximized: bool,
    /// A real fullscreen window: Bloom steps aside entirely.
    pub fullscreen: bool,
}

/// Decides overlap for one surface on `monitor`; mirrors the overlap thread
/// in services.rs. `dock_span` is the dock's expanded bar on screen (x range),
/// `notch_span` the notch's.
pub fn overlap_for(
    top: Option<TopWindow>,
    monitor: Bounds,
    scale: f64,
    surface: Surface,
    dock_span: Option<(i32, i32)>,
    notch_span: Option<(i32, i32)>,
) -> OverlapResult {
    let Some(top) = top else {
        return OverlapResult::default();
    };
    if top.is_shell || top.is_tool {
        return OverlapResult::default();
    }
    let client_fullscreen = top.client.is_some_and(|c| {
        c.left <= monitor.left
            && c.top <= monitor.top
            && c.right >= monitor.right
            && c.bottom >= monitor.bottom
    });
    let covers = top.rect.left <= monitor.left
        && top.rect.top <= monitor.top
        && top.rect.right >= monitor.right
        && top.rect.bottom >= monitor.bottom;
    let maximized_standard = top.maximized && top.has_caption;
    let fullscreen = (client_fullscreen || covers) && !maximized_standard;
    if fullscreen || top.maximized {
        return OverlapResult {
            overlap: true,
            maximized: top.maximized && !fullscreen,
            fullscreen,
        };
    }
    let overlap = match surface {
        Surface::Dock => dock_span.is_some_and(|(left, right)| {
            let trigger_y = monitor.bottom - (56.0 * scale) as i32;
            top.rect.left < right - 4
                && top.rect.right > left + 4
                && top.rect.bottom > trigger_y + 4
        }),
        Surface::Notch => notch_span.is_some_and(|(left, right)| {
            let trigger_y = monitor.top + (36.0 * scale) as i32;
            top.rect.left < right - 4 && top.rect.right > left + 4 && top.rect.top < trigger_y - 4
        }),
    };
    OverlapResult {
        overlap,
        maximized: false,
        fullscreen: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_default_to_the_primary_monitor() {
        assert_eq!(MonitorMode::parse(None), MonitorMode::Primary);
        assert_eq!(MonitorMode::parse(Some("primary")), MonitorMode::Primary);
        assert_eq!(MonitorMode::parse(Some("bogus")), MonitorMode::Primary);
        assert_eq!(MonitorMode::parse(Some("all")), MonitorMode::All);
        assert_eq!(MonitorMode::parse(Some("specific")), MonitorMode::Specific);
    }

    #[test]
    fn monitor_ids_are_label_safe_and_stable() {
        assert_eq!(monitor_id(Some(r"\\.\DISPLAY2"), 1920, 0), "DISPLAY2");
        assert_eq!(monitor_id(Some("Dell U2720Q"), 0, 0), "Dell_U2720Q");
        assert_eq!(monitor_id(None, -1920, 0), "at_1920x0");
        assert_eq!(monitor_id(Some(r"\\.\"), 5, 6), "at5x6");
    }

    #[test]
    fn display_names_prefer_the_windows_display_number() {
        assert_eq!(
            monitor_display_name(0, "DISPLAY3", 2560, 1440),
            "Display 3 (2560\u{d7}1440)"
        );
        assert_eq!(
            monitor_display_name(1, "at1920x0", 1920, 1080),
            "Display 2 (1920\u{d7}1080)"
        );
    }

    #[test]
    fn extra_labels_round_trip_and_skip_the_originals() {
        let label = Surface::Dock.extra_label("DISPLAY2");
        assert_eq!(label, "dock-DISPLAY2");
        assert_eq!(Surface::of_extra_label(&label), Some(Surface::Dock));
        assert_eq!(
            Surface::of_extra_label("main-DISPLAY2"),
            Some(Surface::Notch)
        );
        assert_eq!(Surface::of_extra_label("main"), None);
        assert_eq!(Surface::of_extra_label("dock"), None);
        assert_eq!(Surface::of_extra_label("dock-"), None);
        assert_eq!(Surface::of_extra_label("settings"), None);
    }

    #[test]
    fn edge_gate_arms_only_after_the_dwell() {
        let mut gate = EdgeGate::default();
        assert!(!gate.step(true, 1000, 200));
        assert!(!gate.step(true, 1100, 200));
        assert!(gate.step(true, 1200, 200));
        assert!(gate.step(true, 1250, 200));
        assert!(!gate.step(false, 1300, 200));
        assert_eq!(gate, EdgeGate::default());
        assert!(gate.step(true, 1400, 0));
    }

    fn second_monitor() -> Bounds {
        Bounds::new(1920, 0, 2560, 1440)
    }

    fn notch_input(cursor: (i32, i32), now: i64) -> NotchInput {
        NotchInput {
            cursor,
            now,
            monitor: second_monitor(),
            window_origin: (1920, 0),
            scale: 1.0,
            rect: Some(CssRect {
                x: 1080,
                y: 0,
                width: 400,
                height: 40,
            }),
            fixed_mode: false,
            dwell_ms: 200,
            visible: true,
            drag_held: false,
        }
    }

    #[test]
    fn extra_notch_takes_clicks_only_over_itself() {
        let mut state = InteractionState::default();
        let over = notch_interaction(&mut state, &notch_input((1920 + 1200, 20), 1000));
        assert!(!over.ignore);
        assert!(over.ignore_changed);
        let away = notch_interaction(&mut state, &notch_input((1920 + 100, 700), 1100));
        assert!(away.ignore);
        assert!(away.ignore_changed);
        // A cursor over the primary monitor never touches this notch.
        let elsewhere = notch_interaction(&mut state, &notch_input((500, 2), 1200));
        assert!(elsewhere.ignore);
        assert!(!elsewhere.ignore_changed);
    }

    #[test]
    fn hidden_extra_notch_reveals_after_the_edge_dwell() {
        let mut state = InteractionState::default();
        let mut input = notch_input((1920 + 1250, 1), 1000);
        input.visible = false;
        let first = notch_interaction(&mut state, &input);
        assert_eq!(first.edge_hover, Some(false));
        input.now = 1250;
        let armed = notch_interaction(&mut state, &input);
        assert_eq!(armed.edge_hover, Some(true));
        assert!(!armed.ignore);
    }

    #[test]
    fn fixed_extra_notch_never_arms_the_edge() {
        let mut state = InteractionState::default();
        let mut input = notch_input((1920 + 10, 1), 1000);
        input.fixed_mode = true;
        notch_interaction(&mut state, &input);
        input.now = 5000;
        let effects = notch_interaction(&mut state, &input);
        assert_ne!(effects.edge_hover, Some(true));
        assert!(effects.ignore);
    }

    #[test]
    fn going_click_through_releases_a_stuck_hover() {
        let mut state = InteractionState::default();
        notch_interaction(&mut state, &notch_input((1920 + 1200, 20), 1000));
        state.hovered = true;
        // The webview hover keeps it revealed while the hover lasts...
        let hovered = notch_interaction(&mut state, &notch_input((1920 + 1200, 20), 1100));
        assert!(!hovered.ignore);
        state.hovered = false;
        notch_interaction(&mut state, &notch_input((1920 + 100, 900), 1800));
        state.hovered = true;
        state.last_ignore = 0;
        let left = notch_interaction(&mut state, &notch_input((1920 + 100, 900), 3000));
        assert!(left.ignore);
        assert!(left.pointer_left);
        assert!(!state.hovered);
    }

    fn dock_input(cursor: (i32, i32), now: i64) -> DockInput {
        DockInput {
            cursor,
            now,
            monitor: second_monitor(),
            window: Bounds::new(1920, 1440 - 600, 2560, 600),
            scale: 1.0,
            rect: Some(CssRect {
                x: 980,
                y: 520,
                width: 600,
                height: 64,
            }),
            menu: None,
            dwell_ms: 200,
        }
    }

    #[test]
    fn extra_dock_takes_clicks_over_its_bar_and_open_menu() {
        let mut state = InteractionState::default();
        let bar = dock_interaction(&mut state, &dock_input((1920 + 1200, 1400), 1000));
        assert!(!bar.ignore);
        let above = dock_interaction(&mut state, &dock_input((1920 + 1200, 1000), 1100));
        assert!(above.ignore);
        let mut with_menu = dock_input((1920 + 1200, 1000), 1200);
        with_menu.menu = Some(CssRect {
            x: 1100,
            y: 100,
            width: 200,
            height: 200,
        });
        assert!(!dock_interaction(&mut state, &with_menu).ignore);
    }

    #[test]
    fn extra_dock_bottom_edge_reveals_after_the_dwell() {
        let mut state = InteractionState::default();
        let first = dock_interaction(&mut state, &dock_input((1920 + 1250, 1439), 1000));
        assert_eq!(first.edge_hover, Some(false));
        let armed = dock_interaction(&mut state, &dock_input((1920 + 1250, 1439), 1300));
        assert_eq!(armed.edge_hover, Some(true));
        assert!(!armed.ignore);
        // The hover lingers briefly after leaving, then clears.
        let gone = dock_interaction(&mut state, &dock_input((1920 + 1250, 300), 2000));
        assert_eq!(gone.edge_hover, Some(false));
    }

    #[test]
    fn stepping_aside_drops_hover_and_input() {
        let mut state = InteractionState {
            hovered: true,
            last_ignore: 0,
            last_edge_hover: 1,
            ..Default::default()
        };
        let effects = step_aside(&mut state, true);
        assert!(effects.ignore && effects.ignore_changed && effects.pointer_left);
        assert_eq!(effects.edge_hover, Some(false));
        let again = step_aside(&mut state, true);
        assert!(!again.ignore_changed && again.edge_hover.is_none() && !again.pointer_left);
    }

    fn window(rect: Bounds) -> TopWindow {
        TopWindow {
            is_shell: false,
            is_tool: false,
            rect,
            client: None,
            maximized: false,
            has_caption: true,
        }
    }

    #[test]
    fn overlap_follows_the_window_on_the_same_monitor() {
        let m = second_monitor();
        let dock = Some((1920 + 980, 1920 + 1580));
        let notch = Some((1920 + 1080, 1920 + 1480));
        // Nothing open: nothing to hide for.
        assert_eq!(
            overlap_for(None, m, 1.0, Surface::Dock, dock, notch),
            OverlapResult::default()
        );
        // A window reaching down to the dock.
        let low = window(Bounds::new(1920 + 900, 400, 800, 1000));
        assert!(overlap_for(Some(low), m, 1.0, Surface::Dock, dock, notch).overlap);
        assert!(!overlap_for(Some(low), m, 1.0, Surface::Notch, dock, notch).overlap);
        // A window under the notch.
        let high = window(Bounds::new(1920 + 1000, 10, 600, 400));
        assert!(overlap_for(Some(high), m, 1.0, Surface::Notch, dock, notch).overlap);
        assert!(!overlap_for(Some(high), m, 1.0, Surface::Dock, dock, notch).overlap);
    }

    #[test]
    fn maximized_and_fullscreen_windows_are_told_apart() {
        let m = second_monitor();
        let mut maxed = window(Bounds::new(1920, 0, 2560, 1384));
        maxed.maximized = true;
        let r = overlap_for(Some(maxed), m, 1.0, Surface::Dock, None, None);
        assert_eq!(
            r,
            OverlapResult {
                overlap: true,
                maximized: true,
                fullscreen: false
            }
        );
        let mut game = window(m);
        game.has_caption = false;
        game.client = Some(m);
        let r = overlap_for(Some(game), m, 1.0, Surface::Notch, None, None);
        assert!(r.overlap && r.fullscreen && !r.maximized);
        let mut shell = window(m);
        shell.is_shell = true;
        assert_eq!(
            overlap_for(Some(shell), m, 1.0, Surface::Dock, None, None),
            OverlapResult::default()
        );
    }
}
