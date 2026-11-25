use crate::{
    aqicn::Snapshot,
    config::{Config, TrayDisplay},
    icons,
    level::Level,
    AppState,
};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_positioner::{Position, WindowExt};

/// The dot is drawn on a 36 px canvas that AppKit scales to an 18 pt status
/// item, so 0.56 of the canvas is a ~10 pt dot: readable, but not a marble.
#[cfg(target_os = "macos")]
const DOT_CANVAS: u32 = 36;
#[cfg(target_os = "macos")]
const DOT_DIAMETER: f32 = 0.56;

/// Windows draws the icon in a 16 px slot, so the fill is closer to the edges.
#[cfg(not(target_os = "macos"))]
const DOT_CANVAS: u32 = 32;
#[cfg(not(target_os = "macos"))]
const DOT_DIAMETER: f32 = 0.72;

pub const TRAY_ID: &str = "main";
pub const EVENT_OPEN_SETTINGS: &str = "aqi://open-settings";
pub const EVENT_CHECK_UPDATES: &str = "aqi://check-updates";

/// Menu labels: refresh, check for updates, settings, quit.
fn labels(lang: &str) -> [&'static str; 4] {
    match lang {
        "zh-CN" => ["立即刷新", "检查更新…", "设置…", "退出"],
        "ja" => ["今すぐ更新", "アップデートを確認…", "設定…", "終了"],
        _ => ["Refresh now", "Check for updates…", "Settings…", "Quit"],
    }
}

fn build_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let [refresh, updates, settings, quit] = labels(lang);
    let refresh = MenuItem::with_id(app, "refresh", refresh, true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", updates, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&refresh, &updates, &sep1, &settings, &sep2, &quit])
}

pub fn create(app: &AppHandle, cfg: &Config) -> tauri::Result<()> {
    let menu = build_menu(app, &cfg.resolved_language())?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.notify.notify_one();
                }
            }
            "updates" => {
                let _ = app.emit(EVENT_CHECK_UPDATES, ());
            }
            "settings" => {
                show(app);
                let _ = app.emit(EVENT_OPEN_SETTINGS, ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle(app);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the tray menu after a language change.
pub fn refresh_menu(app: &AppHandle, cfg: &Config) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_menu(app, &cfg.resolved_language()) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn show(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    #[cfg(target_os = "macos")]
    {
        // AppKit objects are main-thread only. A tray click already runs here,
        // so place and reveal inline instead of queueing a turn.
        if objc2::MainThreadMarker::new().is_some() {
            macos_panel::place(app, &win);
            let _ = win.show();
            let _ = win.set_focus();
        } else {
            let handle = app.clone();
            let window = win.clone();
            let _ = app.run_on_main_thread(move || {
                macos_panel::place(&handle, &window);
                let _ = window.show();
                let _ = window.set_focus();
            });
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = win.move_window(Position::TrayCenter);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        show(app);
    }
}

/// What the menu bar shows, given a reading and the user's display settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayText {
    /// The menu bar title (macOS). Empty means "icon only".
    pub title: String,
    /// The hover tooltip / Windows tray tooltip.
    pub tooltip: String,
}

/// aqicn labels a station "English (本地名)", e.g. `Shanghai (上海)`. Split it
/// so the menu bar can show the half its reader recognises. Full-width
/// parentheses appear in some feeds, so both forms are handled.
fn split_station(name: &str) -> (String, Option<String>) {
    let trimmed = name.trim();
    let (open_ch, close_ch) = if trimmed.contains('（') {
        ('（', '）')
    } else {
        ('(', ')')
    };
    if let Some(open) = trimmed.rfind(open_ch) {
        if trimmed.ends_with(close_ch) {
            let base = trimmed[..open].trim().to_string();
            let local = trimmed[open + open_ch.len_utf8()..trimmed.len() - close_ch.len_utf8()]
                .trim()
                .to_string();
            if !local.is_empty() && !base.is_empty() {
                return (base, Some(local));
            }
        }
    }
    (trimmed.to_string(), None)
}

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c as u32,
            0x3040..=0x30FF     // kana
            | 0x3400..=0x4DBF   // CJK ext A
            | 0x4E00..=0x9FFF   // CJK unified
            | 0xF900..=0xFAFF   // compatibility ideographs
        )
    })
}

/// The station name as the menu bar and the panel show it. Chinese and
/// Japanese readers go by the local name in the parentheses; English keeps the
/// Latin one, because that is what the feed spells the place as.
pub fn station_display(name: &str, lang: &str) -> String {
    let trimmed = name.trim();
    let (base, local) = split_station(trimmed);
    match local {
        Some(local) if matches!(lang, "zh-CN" | "ja") && has_cjk(&local) => local,
        // A Latin reader gets the whole feed string, local name included.
        Some(_) => trimmed.to_string(),
        None => base,
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

fn code_value(snap: &Snapshot, code: &str) -> Option<f64> {
    snap.pollutants
        .iter()
        .find(|p| p.code == code)
        .map(|p| p.value)
        .filter(|v| v.is_finite() && *v >= 0.0)
}

fn aqi_value(snap: &Snapshot) -> Option<i32> {
    snap.aqi.filter(|v| *v >= 0)
}

/// "51" for AQI, "12" for PM2.5; falls back to the AQI when PM2.5 is missing,
/// because a blank menu bar is worse than the headline number.
fn metric_text(snap: &Snapshot, metric: &str) -> Option<(String, &'static str)> {
    match metric {
        "pm25" => match code_value(snap, "pm25") {
            Some(v) => Some(((v.round() as i64).to_string(), "PM2.5")),
            None => aqi_value(snap).map(|v| (v.to_string(), "AQI")),
        },
        "none" => None,
        _ => aqi_value(snap).map(|v| (v.to_string(), "AQI")),
    }
}

/// Compose the menu bar title and the tooltip.
///
/// Order is value, level, station — the number people came for stays first and
/// nearest the dot. The level word follows the UI language, so a Chinese menu
/// bar reads `51 · 良` while the English one reads `51 · Moderate`.
pub fn texts(snap: &Snapshot, display: &TrayDisplay, lang: &str) -> TrayText {
    // The title is a menu bar, not a page: keep the station short there and
    // let the tooltip carry the full name.
    let shown = station_display(&snap.station, lang);
    let station = truncate_chars(&shown, 18);
    let tooltip_station = truncate_chars(&shown, 48);
    let pm25 = code_value(snap, "pm25").map(|v| format!("PM2.5 {}", v.round() as i64));
    let aqi = aqi_value(snap);

    // The level describes the AQI reading; with no reading there is no level.
    let level = Level::of(aqi);
    let level_word = if level.is_known() {
        Some(level.short(lang))
    } else {
        None
    };

    // Tooltip: always every useful number, regardless of what the title shows.
    // Windows has no menu bar text at all, so the level belongs here for free.
    let mut tooltip_parts: Vec<String> = Vec::new();
    tooltip_parts.push(match aqi {
        Some(v) => format!("AQI {v}"),
        None => "AQI –".to_string(),
    });
    if let Some(word) = level_word {
        tooltip_parts.push(word.to_string());
    }
    if let Some(pm) = pm25 {
        tooltip_parts.push(pm);
    }
    if !tooltip_station.is_empty() {
        tooltip_parts.push(tooltip_station);
    }
    let tooltip = truncate_chars(&tooltip_parts.join(" · "), 120);

    let title = match metric_text(snap, &display.metric) {
        Some((value, label)) => {
            let mut parts: Vec<String> = Vec::new();
            parts.push(if display.show_label {
                format!("{label} {value}")
            } else {
                value
            });
            if display.show_category {
                if let Some(word) = level_word {
                    parts.push(word.to_string());
                }
            }
            if display.show_station && !station.is_empty() {
                parts.push(station.clone());
            }
            parts.join(" · ")
        }
        None => match (display.metric.as_str(), aqi) {
            // Icon-only: nothing beside the dot while there is a reading.
            ("none", Some(_)) if display.show_icon => String::new(),
            (_, Some(v)) => match (display.show_category, level_word) {
                // No dot, so the text stands in for it: keep the level there too.
                (true, Some(word)) => format!("{v} · {word}"),
                _ => v.to_string(),
            },
            // No reading at all: a dash is clearer than a bare grey dot.
            (_, None) => "–".to_string(),
        },
    };

    TrayText { title, tooltip }
}

pub fn update(app: &AppHandle, snap: &Snapshot, display: &TrayDisplay, lang: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let text = texts(snap, display, lang);
    let color = icons::color_for(snap.aqi);

    #[cfg(debug_assertions)]
    eprintln!("tray: title={:?} tooltip={:?}", text.title, text.tooltip);

    #[cfg(target_os = "macos")]
    {
        if display.show_icon {
            let _ = tray.set_icon(Some(icons::dot_image(color, DOT_CANVAS, DOT_DIAMETER)));
        } else {
            let _ = tray.set_icon(None::<tauri::image::Image>);
        }
        let _ = tray.set_title(Some(text.title.as_str()));
        paint_title(&tray, &text.title, snap.aqi, display.color_text);
        let _ = tray.set_tooltip(Some(text.tooltip.as_str()));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = tray.set_icon(Some(icons::dot_image(color, DOT_CANVAS, DOT_DIAMETER)));
        let _ = tray.set_tooltip(Some(text.tooltip.as_str()));
    }
}

pub fn clear(app: &AppHandle, display: &TrayDisplay, _lang: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    #[cfg(target_os = "macos")]
    {
        if display.show_icon {
            let _ = tray.set_icon(Some(icons::dot_image(
                icons::color_for(None),
                DOT_CANVAS,
                DOT_DIAMETER,
            )));
        } else {
            let _ = tray.set_icon(None::<tauri::image::Image>);
        }
        let _ = tray.set_title(Some("–"));
        paint_title(&tray, "–", None, false);
        let _ = tray.set_tooltip(Some("AQI –"));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = tray.set_icon(Some(icons::dot_image(
            icons::color_for(None),
            DOT_CANVAS,
            DOT_DIAMETER,
        )));
        let _ = tray.set_tooltip(Some("AQI –"));
    }
}

/// Put the panel under the status item, in AppKit's own point space.
///
/// The positioner plugin works in a mixed space: it scales the status item's
/// frame by the menu bar display's backing scale factor and hands a "physical"
/// position to `set_position`, which tao divides back by the scale factor of
/// whichever display the panel currently sits on. With two displays at
/// different scales that round trip does not cancel, so the panel lands at a
/// scaled version of the right place -- often on the other monitor. Reading and
/// writing points directly has no such problem, and it also skips the plugin's
/// cached rect, which only refreshes on mouse events over the icon.
#[cfg(target_os = "macos")]
mod macos_panel {
    use super::TRAY_ID;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSScreen, NSWindow};
    use objc2_foundation::{NSPoint, NSRect};
    use tauri::{AppHandle, WebviewWindow};

    /// The status item's frame in global AppKit points (y up from the bottom of
    /// the primary display), read live from the button's own window.
    fn status_item_frame(app: &AppHandle) -> Option<NSRect> {
        let tray = app.tray_by_id(TRAY_ID)?;
        tray.with_inner_tray_icon(|inner| {
            // The marker cannot be captured by this `Send` closure, and the
            // status item is only readable on the main thread anyway.
            let mtm = MainThreadMarker::new()?;
            let item = inner.ns_status_item()?;
            let button = item.button(mtm)?;
            let window = button.window()?;
            Some(window.frame())
        })
        .ok()
        .flatten()
    }

    fn contains(rect: NSRect, point: NSPoint) -> bool {
        point.x >= rect.origin.x
            && point.x <= rect.origin.x + rect.size.width
            && point.y >= rect.origin.y
            && point.y <= rect.origin.y + rect.size.height
    }

    /// Returns false when the panel could not be placed, so the caller leaves
    /// the window where it is rather than guessing.
    pub fn place(app: &AppHandle, win: &WebviewWindow) -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        let Some(anchor) = status_item_frame(app) else {
            return false;
        };
        let Ok(ptr) = win.ns_window() else {
            return false;
        };
        if ptr.is_null() {
            return false;
        }
        let panel = unsafe { &*ptr.cast::<NSWindow>() };
        let size = panel.frame().size;
        let centre = NSPoint::new(
            anchor.origin.x + anchor.size.width / 2.0,
            anchor.origin.y + anchor.size.height / 2.0,
        );

        // The panel may still be sitting on another display, so go by where
        // the icon is rather than where the window currently is.
        let screen = NSScreen::screens(mtm)
            .iter()
            .find(|s| contains(s.frame(), centre))
            .or_else(|| panel.screen());
        let visible = screen
            .as_ref()
            .map(|s| rect(s.visibleFrame()))
            .unwrap_or(NO_BOUNDS);

        let [left, top] = top_left(
            [
                anchor.origin.x,
                anchor.origin.y,
                anchor.size.width,
                anchor.size.height,
            ],
            [size.width, size.height],
            visible,
        );

        #[cfg(debug_assertions)]
        eprintln!(
            "panel: icon=({:.0},{:.0} {:.0}x{:.0}) panel={:.0}x{:.0} -> top-left=({:.0},{:.0}) screen={:?}",
            anchor.origin.x,
            anchor.origin.y,
            anchor.size.width,
            anchor.size.height,
            size.width,
            size.height,
            left,
            top,
            screen.as_ref().map(|s| s.frame().origin.x),
        );

        panel.setFrameTopLeftPoint(NSPoint::new(left, top));
        true
    }

    fn rect(r: NSRect) -> [f64; 4] {
        [r.origin.x, r.origin.y, r.size.width, r.size.height]
    }

    /// Stand-in bounds when the icon's display cannot be identified: clamp
    /// against nothing rather than dragging the panel to an edge.
    const NO_BOUNDS: [f64; 4] = [-1.0e9, -1.0e9, 2.0e9, 2.0e9];

    /// The panel's top-left corner in AppKit points: centred under the icon,
    /// hanging from its bottom edge, and kept inside `visible` -- the visible
    /// frame of the display the icon is on.
    ///
    /// Inputs and output are `[x, y, width, height]` in AppKit points, except
    /// `panel`, which is `[width, height]`.
    fn top_left(icon: [f64; 4], panel: [f64; 2], visible: [f64; 4]) -> [f64; 2] {
        let centre_x = icon[0] + icon[2] / 2.0;
        let left_min = visible[0];
        let left_max = (visible[0] + visible[2] - panel[0]).max(left_min);
        let top_max = visible[1] + visible[3];
        // A panel taller than the display keeps its top edge on screen.
        let top_min = (visible[1] + panel[1]).min(top_max);

        let left = (centre_x - panel[0] / 2.0).clamp(left_min, left_max);
        let top = icon[1].clamp(top_min, top_max);
        [left, top]
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// A 1920x1080 primary with a 24 pt menu bar: the status item reaches
        /// from y=1050 to y=1074, the visible area stops at y=1050.
        fn primary() -> [f64; 4] {
            [0.0, 0.0, 1920.0, 1050.0]
        }

        const PANEL: [f64; 2] = [340.0, 520.0];

        #[test]
        fn the_panel_hangs_centred_under_the_icon() {
            let [left, top] = top_left([1148.0, 1050.0, 48.0, 30.0], PANEL, primary());
            assert_eq!(left, 1172.0 - 170.0);
            // Its top edge sits on the icon's bottom edge, not the screen's.
            assert_eq!(top, 1050.0);
        }

        #[test]
        fn an_icon_near_the_right_edge_keeps_the_panel_on_the_display() {
            // A display narrower than the panel plus the icon's offset.
            let visible = [0.0, 0.0, 1440.0, 875.0];
            let [left, _] = top_left([1400.0, 875.0, 48.0, 30.0], PANEL, visible);
            assert_eq!(left, 1440.0 - 340.0);
        }

        #[test]
        fn an_icon_on_a_display_to_the_left_keeps_its_own_origins() {
            // Second display 1920 pt wide, placed left of the primary: its
            // AppKit coordinates start at -1920, and so does the clamp.
            let visible = [-1920.0, 0.0, 1920.0, 1050.0];
            let [left, top] = top_left([-1000.0, 1050.0, 48.0, 30.0], PANEL, visible);
            assert_eq!(left, -976.0 - 170.0);
            assert_eq!(top, 1050.0);
            assert!(left >= -1920.0, "panel ran off the left display");
        }

        #[test]
        fn an_icon_on_a_display_above_the_primary_opens_up_there() {
            // A display stacked above the primary occupies y 1080..2160.
            let visible = [0.0, 1080.0, 2560.0, 1410.0];
            let [_, top] = top_left([600.0, 2130.0, 48.0, 30.0], PANEL, visible);
            assert_eq!(top, 2130.0);
            assert!(top > 1080.0, "panel dropped to the primary display");
        }

        #[test]
        fn a_panel_taller_than_the_display_is_pinned_to_its_top() {
            let visible = [0.0, 0.0, 1920.0, 300.0];
            let [_, top] = top_left([900.0, 300.0, 48.0, 30.0], [340.0, 520.0], visible);
            assert_eq!(top, 300.0);
        }
    }
}

/// Repaint the status item's title. `set_title` installs a plain string; on
/// macOS the only way to give the *text* a colour is an attributed title, so
/// the same string is rebuilt with the level colour (or the system label
/// colour when the option is off). Windows has no menu bar text at all.

#[cfg(target_os = "macos")]
fn paint_title(tray: &tauri::tray::TrayIcon, title: &str, aqi: Option<i32>, colored: bool) {
    let title = title.to_string();
    let res = tray.with_inner_tray_icon(move |inner| {
        let Some(item) = inner.ns_status_item() else {
            return;
        };
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            return;
        };
        let Some(button) = item.button(mtm) else {
            return;
        };
        macos_text::paint(&button, &title, aqi, colored);
    });
    if let Err(err) = res {
        #[cfg(debug_assertions)]
        eprintln!("tray: painting the title failed: {err}");
        let _ = err;
    }
}

#[cfg(target_os = "macos")]
mod macos_text {
    use crate::icons;
    use objc2_app_kit::{
        NSAppearanceCustomization, NSButton, NSColor, NSFontAttributeName,
        NSForegroundColorAttributeName,
    };
    use objc2_foundation::{NSAttributedString, NSMutableAttributedString, NSRange, NSString};
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{AnyThread, Message};

    /// Objective-C attribute values are untyped (`AnyObject`); hand AppKit the
    /// object pointer itself, not the Rust wrapper around it.
    fn any<T: Message>(value: &Retained<T>) -> &AnyObject {
        unsafe { &*(Retained::as_ptr(value) as *const AnyObject) }
    }

    pub fn paint(button: &NSButton, title: &str, aqi: Option<i32>, colored: bool) {
        // The menu bar goes dark with the system appearance; a colour that
        // reads on white is invisible on it, so the palette follows along.
        let dark = button.effectiveAppearance().name().to_string().contains("Dark");
        let rgb = if colored {
            icons::text_color(aqi, dark)
        } else {
            None
        };

        let text = NSString::from_str(title);
        let base = NSAttributedString::initWithString(NSAttributedString::alloc(), &text);
        let attributed = NSMutableAttributedString::initWithAttributedString(
            NSMutableAttributedString::alloc(),
            &base,
        );
        let range = NSRange::new(0, attributed.length());
        if range.length > 0 {
            if let Some(font) = button.font() {
                unsafe { attributed.addAttribute_value_range(NSFontAttributeName, any(&font), range) };
            }
            let color = match rgb {
                Some([r, g, b]) => NSColor::colorWithSRGBRed_green_blue_alpha(
                    f64::from(r) / 255.0,
                    f64::from(g) / 255.0,
                    f64::from(b) / 255.0,
                    1.0,
                ),
                None => NSColor::labelColor(),
            };
            unsafe {
                attributed.addAttribute_value_range(NSForegroundColorAttributeName, any(&color), range)
            };
        }
        button.setAttributedTitle(&attributed);
        #[cfg(debug_assertions)]
        report(button, title, rgb);
    }

    /// Read the colour back out of the status item. A screenshot of the menu
    /// bar is not always possible (locked screen, auto-hidden bar), and this is
    /// the stronger check anyway: it shows what AppKit actually kept.
    ///
    /// Never ask a system colour for its components: `labelColor` is dynamic
    /// and raises for `getRed:green:blue:alpha:`.
    #[cfg(debug_assertions)]
    fn report(button: &NSButton, title: &str, expected: Option<[u8; 3]>) {
        unsafe {
            let read = button.attributedTitle();
            let value = read.attribute_atIndex_effectiveRange(
                NSForegroundColorAttributeName,
                0,
                std::ptr::null_mut(),
            );
            let Some(color) = value.and_then(|v| v.downcast_ref::<NSColor>().map(|c| c.retain()))
            else {
                eprintln!("tray: text={title:?} colour=<missing>");
                return;
            };
            match expected {
                None => eprintln!("tray: text={title:?} colour=system label colour"),
                Some(want) => {
                    let (mut r, mut g, mut b, mut a) = (0.0, 0.0, 0.0, 0.0);
                    color.getRed_green_blue_alpha(&mut r, &mut g, &mut b, &mut a);
                    let got = [
                        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                    ];
                    eprintln!(
                        "tray: text={title:?} colour=#{:02x}{:02x}{:02x} expected=#{:02x}{:02x}{:02x} match={}",
                        got[0], got[1], got[2], want[0], want[1], want[2], got == want
                    );
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aqicn::{Pollutant, Snapshot};

    fn snap(aqi: Option<i32>, station: &str, pm25: Option<f64>) -> Snapshot {
        let mut pollutants = Vec::new();
        if let Some(v) = pm25 {
            pollutants.push(Pollutant {
                code: "pm25".into(),
                value: v,
            });
        }
        Snapshot {
            aqi,
            station: station.into(),
            station_url: None,
            dominant: Some("pm25".into()),
            updated: None,
            stale: false,
            pollutants,
            attributions: vec![],
            observed_at: None,
            forecast: vec![],
            fetched_at: 0,
        }
    }

    fn display(metric: &str, label: bool, station: bool, icon: bool) -> TrayDisplay {
        TrayDisplay {
            metric: metric.into(),
            show_label: label,
            show_category: false,
            show_station: station,
            show_icon: icon,
            color_text: false,
        }
    }

    #[test]
    fn default_is_the_bare_number_with_its_level() {
        let t = texts(
            &snap(Some(51), "Shanghai (上海)", Some(12.3)),
            &TrayDisplay::default(),
            "en",
        );
        assert_eq!(t.title, "51 · Moderate");
        assert_eq!(t.tooltip, "AQI 51 · Moderate · PM2.5 12 · Shanghai (上海)");
    }

    #[test]
    fn the_level_word_follows_the_ui_language() {
        let d = TrayDisplay {
            show_station: true,
            ..TrayDisplay::default()
        };
        let reading = snap(Some(51), "Shanghai (上海)", Some(12.3));
        assert_eq!(texts(&reading, &d, "zh-CN").title, "51 · 良 · 上海");
        assert_eq!(texts(&reading, &d, "ja").title, "51 · 普通 · 上海");
        assert_eq!(
            texts(&reading, &d, "en").title,
            "51 · Moderate · Shanghai (上海)"
        );
    }

    #[test]
    fn the_level_can_be_turned_off() {
        let d = display("aqi", false, false, true);
        assert_eq!(texts(&snap(Some(51), "X", None), &d, "zh-CN").title, "51");
    }

    #[test]
    fn label_level_and_station_compose_in_reading_order() {
        let d = TrayDisplay {
            metric: "aqi".into(),
            show_label: true,
            show_category: true,
            show_station: true,
            show_icon: true,
            color_text: false,
        };
        let t = texts(&snap(Some(178), "Shanghai (上海)", Some(120.0)), &d, "en");
        assert_eq!(t.title, "AQI 178 · Unhealthy · Shanghai (上海)");
        assert_eq!(
            t.tooltip,
            "AQI 178 · Unhealthy · PM2.5 120 · Shanghai (上海)"
        );
    }

    #[test]
    fn label_and_station_are_opt_in() {
        let d = display("aqi", true, true, true);
        let t = texts(&snap(Some(51), "Shanghai (上海)", Some(12.3)), &d, "en");
        assert_eq!(t.title, "AQI 51 · Shanghai (上海)");

        let d = display("aqi", true, false, true);
        assert_eq!(
            texts(&snap(Some(51), "Shanghai (上海)", None), &d, "en").title,
            "AQI 51"
        );
    }

    #[test]
    fn pm25_metric_shows_the_particle_number_and_the_aqi_level() {
        let d = TrayDisplay {
            show_category: true,
            ..display("pm25", true, false, true)
        };
        let t = texts(&snap(Some(51), "X", Some(12.6)), &d, "zh-CN");
        assert_eq!(t.title, "PM2.5 13 · 良");
    }

    #[test]
    fn pm25_metric_falls_back_to_aqi_when_the_station_has_no_pm25() {
        let d = display("pm25", true, false, true);
        assert_eq!(texts(&snap(Some(51), "X", None), &d, "en").title, "AQI 51");
    }

    #[test]
    fn no_metric_keeps_the_dot_only() {
        let d = display("none", true, true, true);
        let t = texts(&snap(Some(51), "Shanghai", Some(12.0)), &d, "en");
        assert_eq!(t.title, "");
        // The tooltip still carries the reading and the level.
        assert_eq!(t.tooltip, "AQI 51 · Moderate · PM2.5 12 · Shanghai");
    }

    #[test]
    fn no_metric_without_a_dot_still_shows_the_number() {
        let d = display("none", false, false, false);
        assert_eq!(texts(&snap(Some(51), "X", None), &d, "en").title, "51");
        assert_eq!(texts(&snap(None, "X", None), &d, "en").title, "–");
        // With the level on, the text stands in for the missing dot.
        let d = TrayDisplay {
            show_category: true,
            ..display("none", false, false, false)
        };
        assert_eq!(
            texts(&snap(Some(51), "X", None), &d, "zh-CN").title,
            "51 · 良"
        );
    }

    #[test]
    fn missing_reading_shows_a_dash_and_no_level() {
        let t = texts(&snap(None, "", None), &TrayDisplay::default(), "en");
        assert_eq!(t.title, "–");
        assert_eq!(t.tooltip, "AQI –");
    }

    #[test]
    fn chinese_and_japanese_read_the_name_in_parentheses() {
        assert_eq!(station_display("Shanghai (上海)", "zh-CN"), "上海");
        assert_eq!(station_display("Tokyo (東京)", "ja"), "東京");
        assert_eq!(station_display("Shanghai (上海)", "en"), "Shanghai (上海)");
        assert_eq!(station_display("Jing'an", "zh-CN"), "Jing'an");
        // A Latin local name is no better than the base one.
        assert_eq!(station_display("Pudong (Pudong)", "zh-CN"), "Pudong (Pudong)");
        // Full-width parentheses turn up in some feeds.
        assert_eq!(station_display("Osaka（大阪）", "zh-CN"), "大阪");
        assert_eq!(station_display("", "zh-CN"), "");
        assert_eq!(station_display("  X  ", "en"), "X");
    }

    #[test]
    fn long_station_names_are_truncated_in_both_places() {
        let long = "A".repeat(60);
        let t = texts(
            &snap(Some(51), &long, None),
            &display("aqi", false, true, true),
            "en",
        );
        assert!(t.title.chars().count() <= 24, "{}", t.title);
        assert!(t.title.ends_with('…'));
        assert!(t.tooltip.chars().count() <= 70, "{}", t.tooltip);
    }

    #[test]
    fn extremes_keep_their_level() {
        let d = TrayDisplay::default();
        assert_eq!(
            texts(&snap(Some(0), "X", None), &d, "zh-CN").title,
            "0 · 优"
        );
        assert_eq!(
            texts(&snap(Some(320), "X", None), &d, "zh-CN").title,
            "320 · 严重污染"
        );
        assert_eq!(
            texts(&snap(Some(320), "X", None), &d, "en").title,
            "320 · Hazardous"
        );
    }
}
