//! Multi-monitor placement for the notch and the dock.
//!
//! Each surface has its own monitor setting (see monitor_logic.rs for keys):
//!
//! - `primary` (default): Bloom's original behavior, primary monitor only.
//! - `specific`: the surface's original window ("main" / "dock") moves to the
//!   chosen monitor. Everything that used to assume the primary monitor asks
//!   [`target_monitor`] instead.
//! - `all`: the original window stays on the primary monitor and every other
//!   monitor gets an extra window ("main-<id>" / "dock-<id>") that loads the
//!   same page.
//!
//! An extra window is a full copy of its surface, so it needs what services.rs
//! does for the originals: click-through and hover from the cursor, the edge
//! reveal, an AppBar reservation in fixed mode, and overlap / maximized /
//! fullscreen tracking against the windows on *its own* monitor. That state is
//! kept per window label here, so a webview can't tell it apart from the
//! original and the frontend needs no changes beyond reporting from every
//! notch window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Monitor};
use windows::Win32::Foundation::{HWND, POINT, RECT};

use crate::monitor_logic::{
    self, Bounds, CssRect, DockInput, InteractionEffects, InteractionState, MonitorMode,
    NotchInput, Surface, TopWindow,
};
use crate::state::{OVERLAY_IN_SPLASH, SHUTTING_DOWN};
use crate::types::IntRect;
use crate::utils::{
    get_bloom_scale, get_now_ms, get_setting_str, re_assert_topmost, set_click_through,
};

// ---------------------------------------------------------------------------
// Monitors and settings
// ---------------------------------------------------------------------------

fn id_of(monitor: &Monitor) -> String {
    let pos = monitor.position();
    monitor_logic::monitor_id(monitor.name().map(String::as_str), pos.x, pos.y)
}

fn bounds_of(monitor: &Monitor) -> Bounds {
    let pos = monitor.position();
    let size = monitor.size();
    Bounds::new(pos.x, pos.y, size.width, size.height)
}

fn same_monitor(a: &Monitor, b: &Monitor) -> bool {
    bounds_of(a) == bounds_of(b)
}

pub fn mode(app: &AppHandle, surface: Surface) -> MonitorMode {
    MonitorMode::parse(get_setting_str(app, surface.mode_key()).as_deref())
}

/// The monitor a surface's original window ("main" / "dock") belongs on: the
/// chosen one in `specific` mode while it is connected, the primary otherwise.
pub fn target_monitor(app: &AppHandle, surface: Surface) -> Option<Monitor> {
    if mode(app, surface) == MonitorMode::Specific {
        if let Some(id) = get_setting_str(app, surface.id_key()) {
            let chosen = app
                .available_monitors()
                .ok()
                .and_then(|all| all.into_iter().find(|m| id_of(m) == id));
            if chosen.is_some() {
                return chosen;
            }
        }
    }
    app.primary_monitor().ok().flatten()
}

/// Bounds and scale of each surface's target monitor, refreshed at most once a
/// second: the mouse hook and the overlap thread read it constantly.
struct TargetCache {
    refreshed_ms: i64,
    notch: Option<(Bounds, f64)>,
    dock: Option<(Bounds, f64)>,
}

static TARGET_CACHE: Mutex<TargetCache> = Mutex::new(TargetCache {
    refreshed_ms: 0,
    notch: None,
    dock: None,
});

fn target_cache() -> MutexGuard<'static, TargetCache> {
    TARGET_CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

fn describe(monitor: Option<Monitor>) -> Option<(Bounds, f64)> {
    monitor.map(|m| (bounds_of(&m), m.scale_factor()))
}

/// Bounds and scale of the surface's target monitor (cached, see above).
pub fn target_bounds(app: &AppHandle, surface: Surface) -> Option<(Bounds, f64)> {
    let now = get_now_ms();
    let stale = now - target_cache().refreshed_ms > 1000;
    if stale {
        // Looked up without holding the lock: enumerating monitors can block.
        let notch = describe(target_monitor(app, Surface::Notch));
        let dock = describe(target_monitor(app, Surface::Dock));
        let mut cache = target_cache();
        if notch.is_some() || dock.is_some() {
            cache.notch = notch;
            cache.dock = dock;
            cache.refreshed_ms = now;
        }
    }
    cached_target(surface)
}

/// The last known target bounds, without refreshing (for code with no
/// AppHandle at hand). `None` until the first refresh.
pub fn cached_target(surface: Surface) -> Option<(Bounds, f64)> {
    let cache = target_cache();
    match surface {
        Surface::Notch => cache.notch,
        Surface::Dock => cache.dock,
    }
}

fn invalidate_targets() {
    target_cache().refreshed_ms = 0;
}

#[derive(Serialize)]
pub struct MonitorInfo {
    id: String,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    primary: bool,
}

/// Connected monitors for the settings picker.
#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten();
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, m)| {
            let id = id_of(m);
            let size = m.size();
            let pos = m.position();
            MonitorInfo {
                name: monitor_logic::monitor_display_name(index, &id, size.width, size.height),
                id,
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
                primary: primary.as_ref().is_some_and(|p| same_monitor(p, m)),
            }
        })
        .collect()
}

fn normalized_mode(value: Option<String>, fallback: &str) -> String {
    match value.as_deref() {
        Some("auto-hide") => "smart".to_string(),
        Some(mode) => mode.to_string(),
        None => fallback.to_string(),
    }
}

/// Applies saved monitor settings: moves the original windows to their target
/// monitors and creates or removes the extra windows.
#[tauri::command]
pub async fn apply_monitor_settings(app: AppHandle) {
    invalidate_targets();
    let notch_mode = normalized_mode(get_setting_str(&app, "bloom-notch-mode"), "fixed");
    crate::commands::change_notch_mode(app.clone(), notch_mode).await;
    if crate::utils::dock_enabled() {
        let dock_mode = normalized_mode(get_setting_str(&app, "bloom-dock-mode"), "smart");
        crate::commands::change_dock_mode(app.clone(), dock_mode).await;
    }
    sync(&app);
}

// ---------------------------------------------------------------------------
// Extra windows: state
// ---------------------------------------------------------------------------

struct Extra {
    label: String,
    surface: Surface,
    hwnd: isize,
    monitor: Bounds,
    scale: f64,
    /// Notch box / dock hit area, in the window's CSS pixels.
    rect: Option<CssRect>,
    /// The dock's expanded bar, used for overlap (see `update_dock_rect`).
    overlap_rect: Option<CssRect>,
    /// An open dock menu or popup.
    menu: Option<CssRect>,
    /// The notch is on screen (not hidden by smart/peek mode).
    visible: bool,
    interaction: InteractionState,
    appbar: bool,
    /// A fullscreen app covers this monitor.
    fullscreen: bool,
    last_overlap: Option<bool>,
    last_maximized: Option<bool>,
    last_fullscreen: Option<bool>,
    last_emit_ms: i64,
}

impl Extra {
    fn new(label: String, surface: Surface, monitor: Bounds, scale: f64) -> Extra {
        Extra {
            label,
            surface,
            hwnd: 0,
            monitor,
            scale,
            rect: None,
            overlap_rect: None,
            menu: None,
            visible: true,
            interaction: InteractionState::default(),
            appbar: false,
            fullscreen: false,
            last_overlap: None,
            last_maximized: None,
            last_fullscreen: None,
            last_emit_ms: 0,
        }
    }
}

static EXTRAS: Mutex<Vec<Extra>> = Mutex::new(Vec::new());

fn extras() -> MutexGuard<'static, Vec<Extra>> {
    EXTRAS.lock().unwrap_or_else(|e| e.into_inner())
}

fn with_extra<T>(label: &str, f: impl FnOnce(&mut Extra) -> T) -> Option<T> {
    extras().iter_mut().find(|e| e.label == label).map(f)
}

fn css(rect: IntRect) -> CssRect {
    CssRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

/// Whether a window label is one of the extra notch/dock windows.
pub fn is_extra(label: &str) -> bool {
    Surface::of_extra_label(label).is_some()
}

/// `update_notch_rect` / `update_dock_rect` from an extra window.
pub fn report_rect(label: &str, rect: IntRect, overlap_rect: Option<IntRect>) {
    with_extra(label, |e| {
        e.rect = Some(css(rect));
        if let Some(overlap) = overlap_rect {
            e.overlap_rect = Some(css(overlap));
        }
    });
}

/// `set_notch_hovered` / `set_dock_hovered` from an extra window.
pub fn report_hovered(app: &AppHandle, label: &str, hovered: bool) {
    let notch = with_extra(label, |e| {
        e.interaction.hovered = hovered;
        e.surface == Surface::Notch
    });
    // Re-evaluate at once, as `refresh_main_interaction` does for the
    // original notch: a native drag-out never shows the webview a leave.
    if notch == Some(true) {
        refresh(app);
    }
}

/// `set_notch_visible` from an extra window.
pub fn report_visible(label: &str, visible: bool) {
    with_extra(label, |e| e.visible = visible);
}

/// `set_menu_open` from an extra dock.
pub fn report_menu(label: &str, open: bool, rect: Option<IntRect>) {
    with_extra(label, |e| {
        e.menu = if open {
            Some(rect.map(css).unwrap_or(CssRect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }))
        } else {
            None
        };
    });
}

// ---------------------------------------------------------------------------
// Extra windows: cursor interaction
// ---------------------------------------------------------------------------

fn window_bounds(hwnd: HWND) -> Option<Bounds> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some(Bounds {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

fn left_button_held() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    (unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } as u16 & 0x8000) != 0
}

fn notch_mode_is_fixed(app: &AppHandle) -> bool {
    normalized_mode(get_setting_str(app, "bloom-notch-mode"), "fixed") == "fixed"
}

fn dock_mode_is_fixed(app: &AppHandle) -> bool {
    normalized_mode(get_setting_str(app, "bloom-dock-mode"), "smart") == "fixed"
}

/// Hover and click-through for every extra window, from a cursor position.
/// Called on mouse movement and by the edge watchdog, next to the originals'
/// `update_main_interaction` / `update_dock_interaction`.
pub fn update_interactions(app: &AppHandle, cursor: POINT, now: i64, capture_active: bool) {
    if SHUTTING_DOWN.load(Ordering::Relaxed) {
        return;
    }
    let mut actions: Vec<(String, isize, Surface, InteractionEffects)> = Vec::new();
    {
        let mut list = extras();
        if list.is_empty() {
            return;
        }
        let notch_fixed = notch_mode_is_fixed(app);
        let notch_dwell = crate::services::top_edge_dwell_ms(app);
        let dock_dwell = crate::services::dock_edge_dwell_ms(app);
        let drag_held = left_button_held();
        for e in list.iter_mut() {
            if e.hwnd == 0 {
                continue;
            }
            let hwnd = HWND(e.hwnd as *mut _);
            if !unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) }.as_bool()
            {
                continue;
            }
            let effects = if capture_active || e.fullscreen {
                monitor_logic::step_aside(&mut e.interaction, e.surface == Surface::Notch)
            } else {
                let Some(window) = window_bounds(hwnd) else {
                    continue;
                };
                match e.surface {
                    Surface::Notch => monitor_logic::notch_interaction(
                        &mut e.interaction,
                        &NotchInput {
                            cursor: (cursor.x, cursor.y),
                            now,
                            monitor: e.monitor,
                            window_origin: (window.left, window.top),
                            scale: e.scale,
                            rect: e.rect,
                            fixed_mode: notch_fixed,
                            dwell_ms: notch_dwell,
                            visible: e.visible,
                            drag_held,
                        },
                    ),
                    Surface::Dock => monitor_logic::dock_interaction(
                        &mut e.interaction,
                        &DockInput {
                            cursor: (cursor.x, cursor.y),
                            now,
                            monitor: e.monitor,
                            window,
                            scale: e.scale,
                            rect: e.rect,
                            menu: e.menu,
                            dwell_ms: dock_dwell,
                        },
                    ),
                }
            };
            actions.push((e.label.clone(), e.hwnd, e.surface, effects));
        }
    }

    let drag_held = left_button_held();
    for (label, hwnd, surface, effects) in actions {
        let hwnd = HWND(hwnd as *mut _);
        if effects.ignore_changed && !drag_held {
            re_assert_topmost(hwnd);
        }
        set_click_through(hwnd, effects.ignore);
        if let Some(hovered) = effects.edge_hover {
            let event = match surface {
                Surface::Notch => "notch-edge-hover",
                Surface::Dock => "dock-edge-hover",
            };
            let _ = app.emit_to(label.as_str(), event, hovered);
        }
        if effects.pointer_left {
            let _ = app.emit_to(label.as_str(), "notch-pointer-left", ());
        }
    }
}

/// Re-runs extra-window interaction at the current cursor position.
pub fn refresh(app: &AppHandle) {
    let mut point = POINT::default();
    if unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point) }.is_err() {
        return;
    }
    update_interactions(app, point, get_now_ms(), false);
}

/// Whether an extra dock takes a press at `point`: it isn't click-through and
/// the point is inside its window region (see `dock_takes_press`).
unsafe fn dock_takes_press(hwnd: HWND, point: POINT) -> bool {
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, GetWindowRgn, PtInRegion};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, GetWindowRect, GWL_EXSTYLE, WS_EX_TRANSPARENT,
    };
    if hwnd.0.is_null() || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT.0 != 0 {
        return false;
    }
    let mut bounds = RECT::default();
    if GetWindowRect(hwnd, &mut bounds).is_err() {
        return false;
    }
    let region = CreateRectRgn(0, 0, 0, 0);
    let inside = if GetWindowRgn(hwnd, region).0 == 0 {
        point.x >= bounds.left
            && point.x < bounds.right
            && point.y >= bounds.top
            && point.y < bounds.bottom
    } else {
        PtInRegion(region, point.x - bounds.left, point.y - bounds.top).as_bool()
    };
    let _ = DeleteObject(region.into());
    inside
}

/// From the low-level mouse hook: a press outside an extra dock with an open
/// menu closes that menu, as for the original dock. Returns the labels to
/// notify; the caller emits off the hook thread. Never blocks the hook.
pub fn menus_dismissed_by_press(point: POINT) -> Vec<String> {
    let Ok(mut list) = EXTRAS.try_lock() else {
        return Vec::new();
    };
    let mut dismissed = Vec::new();
    for e in list.iter_mut() {
        if e.surface == Surface::Dock
            && e.menu.is_some()
            && !unsafe { dock_takes_press(HWND(e.hwnd as *mut _), point) }
        {
            e.menu = None;
            dismissed.push(e.label.clone());
        }
    }
    dismissed
}

// ---------------------------------------------------------------------------
// Extra windows: overlap, maximized and fullscreen per monitor
// ---------------------------------------------------------------------------

/// Describes a top-level window as a candidate for "the window on top of its
/// monitor", skipping what the original overlap thread skips: Bloom itself,
/// menus, hidden, minimized and cloaked windows, and (unless focused)
/// always-on-top and tool windows, which float over apps without covering
/// them.
unsafe fn candidate(hwnd: HWND, foreground: HWND, my_pid: u32) -> Option<(Bounds, TopWindow)> {
    use windows::Win32::Foundation::POINT as WinPoint;
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
    };
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameA, GetClientRect, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId,
        IsIconic, IsWindowVisible, IsZoomed, GWL_EXSTYLE, GWL_STYLE, WS_CAPTION, WS_EX_TOOLWINDOW,
        WS_EX_TOPMOST, WS_MAXIMIZE,
    };

    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let mut class_buf = [0u8; 256];
    let len = GetClassNameA(hwnd, &mut class_buf).max(0) as usize;
    let class = std::str::from_utf8(&class_buf[..len]).unwrap_or("");
    if pid == my_pid || class.contains("Bloom") || class == "#32768" {
        return None;
    }
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut _ as *mut _,
        std::mem::size_of::<u32>() as u32,
    )
    .is_ok()
        && cloaked != 0
    {
        return None;
    }
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let focused = hwnd == foreground;
    let is_tool = ex_style & WS_EX_TOOLWINDOW.0 != 0;
    if !focused && (is_tool || ex_style & WS_EX_TOPMOST.0 != 0) {
        return None;
    }

    let mut rect = RECT::default();
    let has_rect = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut rect as *mut _ as *mut _,
        std::mem::size_of::<RECT>() as u32,
    )
    .is_ok()
        || GetWindowRect(hwnd, &mut rect).is_ok();
    if !has_rect || rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }

    let h_monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(h_monitor, &mut info).as_bool() {
        return None;
    }
    let monitor = Bounds {
        left: info.rcMonitor.left,
        top: info.rcMonitor.top,
        right: info.rcMonitor.right,
        bottom: info.rcMonitor.bottom,
    };

    let mut client = None;
    let mut client_rect = RECT::default();
    if GetClientRect(hwnd, &mut client_rect).is_ok() {
        let mut top_left = WinPoint {
            x: client_rect.left,
            y: client_rect.top,
        };
        let mut bottom_right = WinPoint {
            x: client_rect.right,
            y: client_rect.bottom,
        };
        if ClientToScreen(hwnd, &mut top_left).as_bool()
            && ClientToScreen(hwnd, &mut bottom_right).as_bool()
        {
            client = Some(Bounds {
                left: top_left.x,
                top: top_left.y,
                right: bottom_right.x,
                bottom: bottom_right.y,
            });
        }
    }

    let is_desktop = class == "Progman" || class == "WorkerW";
    let is_taskbar = class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd";
    let is_start = (class == "Windows.UI.Core.CoreWindow" || class == "SimpleWindow")
        && crate::services::is_start_menu_process(pid);

    Some((
        monitor,
        TopWindow {
            is_shell: is_desktop || is_taskbar || is_start,
            is_tool,
            rect: Bounds {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            },
            client,
            maximized: IsZoomed(hwnd).as_bool() || style & WS_MAXIMIZE.0 != 0,
            has_caption: style & WS_CAPTION.0 != 0,
        },
    ))
}

/// The top window on each of `monitors`, walking the Z-order from the top.
unsafe fn top_windows(monitors: &[Bounds]) -> Vec<Option<TopWindow>> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetTopWindow, GetWindow, GW_HWNDNEXT,
    };
    let mut found: Vec<Option<TopWindow>> = vec![None; monitors.len()];
    let mut assigned = 0;
    let my_pid = std::process::id();
    let foreground = GetForegroundWindow();
    let mut hwnd = GetTopWindow(None).unwrap_or_default();
    let mut checked = 0;
    while !hwnd.is_invalid() && checked < 1024 && assigned < monitors.len() {
        checked += 1;
        if let Some((monitor, top)) = candidate(hwnd, foreground, my_pid) {
            if let Some(index) = monitors.iter().position(|m| *m == monitor) {
                if found[index].is_none() {
                    found[index] = Some(top);
                    assigned += 1;
                }
            }
        }
        hwnd = GetWindow(hwnd, GW_HWNDNEXT).unwrap_or_default();
    }
    found
}

/// Screen x-range of a CSS rect inside a window at `origin_x`.
fn screen_span(origin_x: i32, rect: CssRect, scale: f64) -> (i32, i32) {
    let left = origin_x + (rect.x as f64 * scale) as i32;
    (left, left + (rect.width as f64 * scale) as i32)
}

fn overlap_tick(app: &AppHandle) {
    let mut monitors: Vec<Bounds> = Vec::new();
    for e in extras().iter() {
        if !monitors.contains(&e.monitor) {
            monitors.push(e.monitor);
        }
    }
    if monitors.is_empty() {
        return;
    }
    let tops = unsafe { top_windows(&monitors) };
    let now = get_now_ms();
    let mut emits: Vec<(String, &'static str, bool)> = Vec::new();
    {
        let mut list = extras();
        for e in list.iter_mut() {
            if e.hwnd == 0 {
                continue;
            }
            let hwnd = HWND(e.hwnd as *mut _);
            let top = monitors
                .iter()
                .position(|m| *m == e.monitor)
                .and_then(|index| tops[index]);
            let origin_x = window_bounds(hwnd)
                .map(|w| w.left)
                .unwrap_or(e.monitor.left);
            let (dock_span, notch_span) = match e.surface {
                Surface::Dock => (
                    e.overlap_rect
                        .or(e.rect)
                        .map(|r| screen_span(origin_x, r, e.scale)),
                    None,
                ),
                Surface::Notch => (None, e.rect.map(|r| screen_span(origin_x, r, e.scale))),
            };
            let result = monitor_logic::overlap_for(
                top, e.monitor, e.scale, e.surface, dock_span, notch_span,
            );
            e.fullscreen = result.fullscreen;
            // Re-sent every few seconds, like the originals, so a webview that
            // missed an event (reload, sleep) resyncs.
            let periodic = now - e.last_emit_ms >= 3000;
            if periodic {
                e.last_emit_ms = now;
            }
            match e.surface {
                Surface::Dock => {
                    let visible =
                        unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) }
                            .as_bool();
                    let overlap = result.overlap && visible;
                    if periodic || e.last_overlap != Some(overlap) {
                        e.last_overlap = Some(overlap);
                        emits.push((e.label.clone(), "dock-overlap", overlap));
                    }
                    if periodic || e.last_maximized != Some(result.maximized) {
                        e.last_maximized = Some(result.maximized);
                        emits.push((e.label.clone(), "dock-maximized", result.maximized));
                    }
                }
                Surface::Notch => {
                    if periodic || e.last_overlap != Some(result.overlap) {
                        e.last_overlap = Some(result.overlap);
                        emits.push((e.label.clone(), "notch-overlap", result.overlap));
                    }
                }
            }
            if e.last_fullscreen != Some(result.fullscreen) {
                e.last_fullscreen = Some(result.fullscreen);
                emits.push((e.label.clone(), "visibility-change", !result.fullscreen));
            }
        }
    }
    for (label, event, value) in emits {
        let _ = app.emit_to(label.as_str(), event, value);
    }
}

static OVERLAP_WATCH_STARTED: AtomicBool = AtomicBool::new(false);

fn ensure_overlap_watch(app: &AppHandle) {
    if OVERLAP_WATCH_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(200));
        if SHUTTING_DOWN.load(Ordering::Relaxed) {
            continue;
        }
        overlap_tick(&app);
    });
}

// ---------------------------------------------------------------------------
// Extra windows: placement and AppBars
// ---------------------------------------------------------------------------

/// Reserves a strip along `edge` of `monitor` for `hwnd` as an AppBar and
/// returns the rectangle the shell granted (or the requested one).
unsafe fn reserve(
    hwnd: HWND,
    edge: u32,
    monitor: Bounds,
    thickness: i32,
    registered: &mut bool,
) -> Bounds {
    use windows::Win32::UI::Shell::{
        SHAppBarMessage, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
    };
    let requested = if edge == ABE_TOP {
        RECT {
            left: monitor.left,
            top: monitor.top,
            right: monitor.right,
            bottom: monitor.top + thickness,
        }
    } else {
        RECT {
            left: monitor.left,
            top: monitor.bottom - thickness,
            right: monitor.right,
            bottom: monitor.bottom,
        }
    };
    let mut abd = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    if !*registered {
        let _ = SHAppBarMessage(ABM_REMOVE, &mut abd);
        if SHAppBarMessage(ABM_NEW, &mut abd) == 0 {
            crate::diagnostics::record("monitor_appbar", "registration failed");
        } else {
            *registered = true;
        }
    }
    if *registered {
        abd.uEdge = edge;
        abd.rc = requested;
        SHAppBarMessage(ABM_QUERYPOS, &mut abd);
        SHAppBarMessage(ABM_SETPOS, &mut abd);
        if abd.rc.right > abd.rc.left {
            return Bounds {
                left: abd.rc.left,
                top: abd.rc.top,
                right: abd.rc.right,
                bottom: abd.rc.bottom,
            };
        }
    }
    Bounds {
        left: requested.left,
        top: requested.top,
        right: requested.right,
        bottom: requested.bottom,
    }
}

enum Placement {
    Done,
    Retry,
    Gone,
}

/// Logical height of a surface's window, from tauri.conf.json.
fn configured_height(app: &AppHandle, surface: Surface) -> f64 {
    app.config()
        .app
        .windows
        .iter()
        .find(|c| c.label == surface.base_label())
        .map(|c| c.height)
        .unwrap_or(match surface {
            Surface::Notch => 420.0,
            Surface::Dock => 600.0,
        })
}

fn place_now(app: &AppHandle, label: &str) -> Placement {
    use windows::Win32::UI::Shell::{ABE_BOTTOM, ABE_TOP};
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER,
    };

    let Some(window) = app.get_webview_window(label) else {
        return Placement::Gone;
    };
    let Some((surface, monitor, scale, mut registered)) =
        with_extra(label, |e| (e.surface, e.monitor, e.scale, e.appbar))
    else {
        return Placement::Gone;
    };
    let Ok(hwnd) = window.hwnd() else {
        return Placement::Retry;
    };
    if surface == Surface::Dock && !crate::utils::dock_enabled() {
        return Placement::Gone;
    }
    let bloom_scale = get_bloom_scale(app);
    re_assert_topmost(hwnd);

    let (fixed, edge, reserved_px, window_height) = match surface {
        Surface::Notch => (
            notch_mode_is_fixed(app),
            ABE_TOP,
            (40.0 * bloom_scale * scale) as i32,
            (420.0 * bloom_scale * scale) as i32,
        ),
        Surface::Dock => (
            dock_mode_is_fixed(app),
            ABE_BOTTOM,
            (56.0 * bloom_scale * scale) as i32,
            (configured_height(app, Surface::Dock) * scale) as i32,
        ),
    };
    if window_height <= 10 || monitor.width() <= 0 {
        return Placement::Retry;
    }

    let span = if fixed {
        unsafe { reserve(hwnd, edge, monitor, reserved_px, &mut registered) }
    } else {
        // Smart and peek never hold a reservation; remove it unconditionally,
        // as for the originals, in case the shell still has one.
        crate::services::unregister_appbar_native(hwnd);
        registered = false;
        monitor
    };
    with_extra(label, |e| e.appbar = registered);

    let y = match surface {
        Surface::Notch => monitor.top,
        Surface::Dock => monitor.bottom - window_height,
    };
    let positioned = unsafe {
        SetWindowPos(
            hwnd,
            None,
            span.left,
            y,
            span.width(),
            window_height,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    }
    .is_ok();
    if !positioned {
        return Placement::Retry;
    }
    re_assert_topmost(hwnd);
    if !window.is_visible().unwrap_or(false) && window.show().is_err() {
        return Placement::Retry;
    }
    // show() lets tao rewrite the extended style; restore click-through until
    // the next cursor update decides.
    let interactive = with_extra(label, |e| e.interaction.last_ignore == 0).unwrap_or(false);
    set_click_through(hwnd, !interactive);
    Placement::Done
}

/// Places an extra window on its monitor, retrying while it isn't ready.
fn schedule_place(app: &AppHandle, label: &str, delay_ms: u64) {
    let app = app.clone();
    let label = label.to_string();
    tauri::async_runtime::spawn(async move {
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        for _ in 0..10 {
            if SHUTTING_DOWN.load(Ordering::Relaxed) {
                return;
            }
            match place_now(&app, &label) {
                Placement::Done | Placement::Gone => return,
                Placement::Retry => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
    });
}

/// Re-places every extra window of a surface, after its mode (fixed, smart,
/// peek) or Bloom's scale changed.
pub fn relayout(app: &AppHandle, surface: Option<Surface>) {
    let labels: Vec<String> = extras()
        .iter()
        .filter(|e| surface.is_none_or(|s| e.surface == s))
        .map(|e| e.label.clone())
        .collect();
    for label in labels {
        schedule_place(app, &label, 0);
    }
}

// ---------------------------------------------------------------------------
// Extra windows: lifecycle
// ---------------------------------------------------------------------------

static SYNC_LOCK: Mutex<()> = Mutex::new(());

fn create(app: &AppHandle, label: String, surface: Surface, monitor: &Monitor) {
    let bounds = bounds_of(monitor);
    let scale = monitor.scale_factor();
    extras().push(Extra::new(label.clone(), surface, bounds, scale));

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let forget = |label: &str| extras().retain(|e| e.label != label);
        let Some(mut config) = app
            .config()
            .app
            .windows
            .iter()
            .find(|c| c.label == surface.base_label())
            .cloned()
        else {
            forget(&label);
            return;
        };
        config.label = label.clone();
        config.visible = false;
        config.x = Some(bounds.left as f64 / scale);
        config.y = Some(bounds.top as f64 / scale);
        config.width = bounds.width() as f64 / scale;

        let built = tauri::WebviewWindowBuilder::from_config(&app, &config).and_then(|b| b.build());
        let window = match built {
            Ok(window) => window,
            Err(error) => {
                crate::diagnostics::record("monitor_window", &format!("{label}: {error}"));
                forget(&label);
                return;
            }
        };
        // Removed (monitor unplugged, setting changed) while it was building.
        if with_extra(&label, |_| ()).is_none() {
            let _ = window.destroy();
            return;
        }
        // Layered from the start, like the originals (see main.rs).
        let _ = window.set_ignore_cursor_events(true);
        if let Ok(hwnd) = window.hwnd() {
            set_click_through(hwnd, true);
            with_extra(&label, |e| e.hwnd = hwnd.0 as isize);
        }
        let events_app = app.clone();
        let events_label = label.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
            tauri::WindowEvent::ScaleFactorChanged { .. } => {
                schedule_place(&events_app, &events_label, 400);
            }
            _ => {}
        });

        // Don't paint over the first-run splash.
        let started = Instant::now();
        while OVERLAY_IN_SPLASH.load(Ordering::Relaxed)
            && started.elapsed() < Duration::from_secs(8)
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        schedule_place(&app, &label, 0);
        // A move across monitors with a different DPI resizes the window once
        // Windows reports the new scale; place it again after that settles.
        schedule_place(&app, &label, 700);
        ensure_overlap_watch(&app);
    });
}

fn destroy(app: &AppHandle, label: &str) {
    let removed = {
        let mut list = extras();
        list.iter()
            .position(|e| e.label == label)
            .map(|index| list.remove(index))
    };
    if let Some(e) = removed {
        if e.hwnd != 0 {
            crate::services::unregister_appbar_native(HWND(e.hwnd as *mut _));
        }
    }
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.destroy();
    }
}

/// Creates, moves or removes the extra windows to match the settings, the
/// dock's on/off state and the connected monitors. Safe to call repeatedly:
/// existing windows are only re-placed.
pub fn sync(app: &AppHandle) {
    if SHUTTING_DOWN.load(Ordering::Relaxed) {
        return;
    }
    let _guard = SYNC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    invalidate_targets();
    let monitors = app.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return;
    }
    let primary = app.primary_monitor().ok().flatten();

    let mut wanted: Vec<(String, Surface, Monitor)> = Vec::new();
    for surface in [Surface::Notch, Surface::Dock] {
        if surface == Surface::Dock && !crate::utils::dock_enabled() {
            continue;
        }
        if mode(app, surface) != MonitorMode::All {
            continue;
        }
        for monitor in &monitors {
            if primary.as_ref().is_some_and(|p| same_monitor(p, monitor)) {
                continue;
            }
            wanted.push((
                surface.extra_label(&id_of(monitor)),
                surface,
                monitor.clone(),
            ));
        }
    }

    let mut existing: Vec<String> = extras().iter().map(|e| e.label.clone()).collect();
    // Windows left behind without state (should not happen) are removed too.
    for label in app.webview_windows().into_keys() {
        if is_extra(&label) && !existing.contains(&label) {
            existing.push(label);
        }
    }
    for label in &existing {
        if !wanted.iter().any(|(w, _, _)| w == label) {
            destroy(app, label);
        }
    }

    for (label, surface, monitor) in wanted {
        let bounds = bounds_of(&monitor);
        let scale = monitor.scale_factor();
        let known = with_extra(&label, |e| {
            e.monitor = bounds;
            e.scale = scale;
        })
        .is_some();
        if known {
            schedule_place(app, &label, 0);
        } else {
            create(app, label, surface, &monitor);
        }
    }
    let _ = app.emit("monitors-changed", ());
}

/// Explorer restarted and forgot every AppBar: the next placement registers
/// the extra windows again.
pub fn forget_appbars() {
    for e in extras().iter_mut() {
        e.appbar = false;
    }
}

/// Removes every extra window's AppBar reservation (quit / restart).
pub fn release_all() {
    for e in extras().iter_mut() {
        if e.hwnd != 0 {
            crate::services::unregister_appbar_native(HWND(e.hwnd as *mut _));
        }
        e.appbar = false;
    }
}
