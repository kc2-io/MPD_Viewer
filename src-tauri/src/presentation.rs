//! Native containers and retained surfaces, independent of selection policy.
use crate::{layout, model::ViewerLayout};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Rect, Window};

/// 0 = active, 1 = Stop, 2 = superseded by layout/selection changes.
pub type Cancel = Arc<AtomicU8>;
#[derive(Debug)]
pub struct MoveFailure {
    pub message: String,
    pub source_intact: bool,
}
impl From<String> for MoveFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            source_intact: true,
        }
    }
}
impl From<&str> for MoveFailure {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

pub struct MoveOutcome {
    pub grid: Option<String>,
    pub containers: Vec<String>,
    pub sources: Vec<(String, Window, Rect)>,
}

fn screen(app: &AppHandle) -> Result<(f64, f64, tauri::PhysicalPosition<i32>, f64), String> {
    // The locked runtime converts Tao monitor handles after returning its getter.
    // That conversion calls GTK/AppKit and must also run on the event thread.
    let app_clone = app.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(screen_on_main(&app_clone));
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Display observation timed out".to_owned())?
}
fn screen_on_main(
    app: &AppHandle,
) -> Result<(f64, f64, tauri::PhysicalPosition<i32>, f64), String> {
    let monitor = app
        .get_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("No display is available for Grid.")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return Err("Invalid display scale.".into());
    }
    Ok((
        area.size.width as f64 / scale,
        (area.size.height as f64 / scale - 40.0).max(1.0),
        area.position,
        scale,
    ))
}

pub fn preflight(app: &AppHandle, count: usize, embedded: bool) -> Result<(), String> {
    let (w, h, _, _) = screen(app)?;
    layout::cells_with_min(
        count,
        w,
        h,
        if embedded { 800.0 } else { layout::MIN_WIDTH },
        if embedded { 540.0 } else { layout::MIN_HEIGHT },
    )
    .map(|_| ())
}

// Matches ui/style.css --panel-line in the manager's light/dark themes.
pub fn divider_color(theme: tauri::Theme) -> tauri::window::Color {
    match theme {
        tauri::Theme::Dark => tauri::window::Color(0x4d, 0x71, 0x83, 255),
        _ => tauri::window::Color(0x6f, 0x8a, 0x98, 255),
    }
}
pub fn update_divider_theme(window: &Window, theme: tauri::Theme) -> Result<(), String> {
    window
        .set_background_color(Some(divider_color(theme)))
        .map_err(|e| e.to_string())
}

// Tao's Linux SetTheme event uses a dummy window ID, so it is not delivered
// through Tauri's per-window callback. Observe the GTK setting directly.
#[cfg(target_os = "linux")]
pub fn watch_divider_theme(app: &AppHandle) {
    use gtk::prelude::*;
    if let Some(settings) = gtk::Settings::default() {
        let app = app.clone();
        settings.connect_gtk_application_prefer_dark_theme_notify(move |settings| {
            let fallback = if settings.is_gtk_application_prefer_dark_theme() {
                tauri::Theme::Dark
            } else {
                tauri::Theme::Light
            };
            let theme = app
                .get_window("main")
                .and_then(|window| window.theme().ok())
                .unwrap_or(fallback);
            for (label, window) in app.windows() {
                if label.starts_with("viewer-grid-") {
                    if let Err(error) = update_divider_theme(&window, theme) {
                        eprintln!("Grid theme update failed: {error}");
                    }
                }
            }
        });
    }
}

pub fn container(app: &AppHandle, label: &str, title: &str, grid: bool) -> Result<Window, String> {
    #[cfg(feature = "e2e-tests")]
    if crate::e2e::fault("fail_layout_prepare") {
        return Err("Injected fixture container creation failure".into());
    }
    let (w, h, pos, scale) = screen(app)?;
    let mut builder = tauri::window::WindowBuilder::new(app, label);
    if grid {
        let theme = app
            .get_window("main")
            .ok_or("Manager is unavailable")?
            .theme()
            .map_err(|e| e.to_string())?;
        builder = builder.background_color(divider_color(theme));
    }
    let window = builder
        .title(title)
        .inner_size(
            if grid { w.min(1440.0) } else { 1180.0 },
            if grid { h.min(960.0) } else { 720.0 },
        )
        .min_inner_size(layout::MIN_WIDTH, layout::MIN_HEIGHT)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?;
    // Fit new windows to the available monitor without changing selection.
    let configured = (|| -> Result<(), String> {
        window
            .set_size(LogicalSize::new(
                if grid { w.min(1440.0) } else { w.min(1180.0) },
                if grid { h.min(960.0) } else { h.min(720.0) },
            ))
            .map_err(|e| e.to_string())?;
        window
            .set_position(LogicalPosition::new(
                pos.x as f64 / scale,
                pos.y as f64 / scale,
            ))
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = configured {
        let _ = window.destroy();
        return Err(error);
    }
    Ok(window)
}

pub fn arrange(app: &AppHandle, grid: &str, labels: &[String]) -> Result<Vec<Rect>, String> {
    let window = app
        .get_window(grid)
        .ok_or("Grid window is no longer available.")?;
    let mut size = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
    let (available_w, available_h, work_pos, work_scale) = screen(app)?;
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let offscreen = position.x < work_pos.x
        || position.y < work_pos.y
        || position.x as f64 + size.width * scale
            > work_pos.x as f64 + available_w * work_scale + 1.0
        || position.y as f64 + size.height * scale
            > work_pos.y as f64 + (available_h + 40.0) * work_scale + 1.0;
    if offscreen || size.width > available_w || size.height > available_h {
        window
            .set_min_size(None::<LogicalSize<f64>>)
            .map_err(|e| e.to_string())?;
        size = LogicalSize::new(size.width.min(available_w), size.height.min(available_h));
        window
            .set_position(LogicalPosition::new(
                work_pos.x as f64 / work_scale,
                work_pos.y as f64 / work_scale,
            ))
            .map_err(|e| e.to_string())?;
        window.set_size(size).map_err(|e| e.to_string())?;
    }
    let embedded = labels.iter().any(|label| label.starts_with("player-"));
    let (min_width, min_height) = if embedded {
        (800.0, 540.0)
    } else {
        (layout::MIN_WIDTH, layout::MIN_HEIGHT)
    };
    let bounds = match layout::cells_with_min(
        labels.len(),
        size.width,
        size.height,
        min_width,
        min_height,
    ) {
        Ok(bounds) => bounds,
        Err(_) => {
            let (w, h, pos, scale) = screen(app)?;
            let bounds = layout::cells_with_min(labels.len(), w, h, min_width, min_height)?;
            window
                .set_position(LogicalPosition::new(
                    pos.x as f64 / scale,
                    pos.y as f64 / scale,
                ))
                .map_err(|e| e.to_string())?;
            window
                .set_size(LogicalSize::new(w, h))
                .map_err(|e| e.to_string())?;
            size = LogicalSize::new(w, h);
            bounds
        }
    };
    // Use the current row/column minimum to prevent ordinary undersizing.
    if let Some(first) = bounds.first() {
        let cell = first.size.to_logical::<f64>(1.0);
        let columns = ((size.width + layout::DIVIDER) / (cell.width + layout::DIVIDER)).round();
        let rows = ((size.height + layout::DIVIDER) / (cell.height + layout::DIVIDER)).round();
        window
            .set_min_size(Some(LogicalSize::new(
                columns * min_width + (columns - 1.0) * layout::DIVIDER,
                rows * min_height + (rows - 1.0) * layout::DIVIDER,
            )))
            .map_err(|e| e.to_string())?;
    }
    for (label, bounds) in labels.iter().zip(&bounds) {
        if let Some(webview) = app.get_webview(label) {
            if webview.window().label() == grid {
                set_bounds(&webview, *bounds)?;
            }
        }
    }
    window
        .set_title(&format!("MPD Viewer · {} viewers", labels.len()))
        .map_err(|e| e.to_string())?;
    Ok(bounds)
}

/// Locked Linux Wry uses a GtkBox and ignores cell coordinates there. Keep
/// Tauri's native ownership, but place grid widgets in a native GtkFixed.
/// This changes neither browser security nor document content.
pub fn set_bounds(view: &tauri::Webview, bounds: Rect) -> Result<(), String> {
    view.set_bounds(bounds).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    {
        let window = view.window();
        let (tx, rx) = std::sync::mpsc::channel();
        view.with_webview(move |platform| {
            use gtk::prelude::*;
            let result = (|| -> Result<(), String> {
                let widget = platform.inner();
                if window.label().starts_with("viewer-grid-") {
                    let vbox = window.default_vbox().map_err(|e| e.to_string())?;
                    let fixed = vbox
                        .children()
                        .into_iter()
                        .find(|child| child.widget_name() == "mpd-native-grid")
                        .and_then(|child| child.downcast::<gtk::Fixed>().ok())
                        .unwrap_or_else(|| {
                            let fixed = gtk::Fixed::new();
                            fixed.set_widget_name("mpd-native-grid");
                            vbox.pack_start(&fixed, true, true, 0);
                            fixed.show();
                            fixed
                        });
                    if widget.parent().as_ref() != Some(fixed.upcast_ref()) {
                        if let Some(parent) = widget
                            .parent()
                            .and_then(|p| p.downcast::<gtk::Container>().ok())
                        {
                            parent.remove(&widget);
                        }
                        fixed.put(&widget, 0, 0);
                    }
                    let pos = bounds.position.to_logical::<f64>(1.0);
                    let size = bounds.size.to_logical::<f64>(1.0);
                    widget.set_size_request(size.width.floor() as i32, size.height.floor() as i32);
                    fixed.move_(&widget, pos.x.floor() as i32, pos.y.floor() as i32);
                    widget.show();
                } else {
                    widget.set_size_request(-1, -1);
                }
                Ok(())
            })();
            let _ = tx.send(result);
        })
        .map_err(|e| e.to_string())?;
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "Native grid bounds did not complete.")??;
    }
    Ok(())
}

/// Read actual native allocation, rather than Linux Wry's zero-origin bounds.
pub fn bounds(view: &tauri::Webview) -> Result<Rect, String> {
    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        view.with_webview(move |platform| {
            use gtk::prelude::*;
            let allocation = platform.inner().allocation();
            let _ = tx.send(Rect {
                position: LogicalPosition::new(allocation.x(), allocation.y()).into(),
                size: LogicalSize::new(allocation.width(), allocation.height()).into(),
            });
        })
        .map_err(|e| e.to_string())?;
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "Native bounds inspection did not complete.".into())
    }
    #[cfg(windows)]
    {
        // Locked Wry's child reparent leaves its cached parent unchanged, so
        // bounds() maps coordinates against a retired source. Inspect the HWND's
        // actual parent instead; this also makes rollback snapshots reliable.
        let window = view.window();
        let (tx, rx) = std::sync::mpsc::channel();
        view.with_webview(move |platform| {
            use windows::Win32::{
                Foundation::{GetLastError, SetLastError, ERROR_SUCCESS, HWND, POINT, RECT},
                Graphics::Gdi::MapWindowPoints,
                UI::WindowsAndMessaging::{GetClientRect, GetParent},
            };
            let result = (|| -> Result<Rect, String> {
                let mut hwnd = HWND::default();
                unsafe { platform.controller().ParentWindow(&mut hwnd) }
                    .map_err(|e| e.to_string())?;
                let parent = unsafe { GetParent(hwnd) }.map_err(|e| e.to_string())?;
                if parent != window.hwnd().map_err(|e| e.to_string())? {
                    return Err("Native child belongs to another container".into());
                }
                let mut rect = RECT::default();
                unsafe { GetClientRect(hwnd, &mut rect) }.map_err(|e| e.to_string())?;
                let mut origin = [POINT {
                    x: rect.left,
                    y: rect.top,
                }];
                unsafe { SetLastError(ERROR_SUCCESS) };
                let mapped = unsafe { MapWindowPoints(Some(hwnd), Some(parent), &mut origin) };
                if mapped == 0 && unsafe { GetLastError() } != ERROR_SUCCESS {
                    return Err("Native child coordinate mapping failed".into());
                }
                if rect.right <= rect.left || rect.bottom <= rect.top {
                    return Err("Native child has no usable bounds".into());
                }
                Ok(Rect {
                    position: tauri::PhysicalPosition::new(origin[0].x, origin[0].y).into(),
                    size: tauri::PhysicalSize::new(
                        (rect.right - rect.left) as u32,
                        (rect.bottom - rect.top) as u32,
                    )
                    .into(),
                })
            })();
            let _ = tx.send(result);
        })
        .map_err(|e| e.to_string())?;
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "Native bounds inspection did not complete".to_owned())?
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        view.bounds().map_err(|e| e.to_string())
    }
}

pub fn focus(app: &AppHandle, label: &str) -> Result<(), String> {
    let view = app
        .get_webview(label)
        .ok_or("Viewer is no longer available.")?;
    let window = view.window();
    window
        .show()
        .and_then(|_| window.unminimize())
        .and_then(|_| window.set_focus())
        .and_then(|_| view.set_focus())
        .map_err(|e| e.to_string())
}

/// Completion originates on the event thread, after locked runtime-wry has
/// synchronously removed the native renderer. Registry absence is insufficient.
pub fn close(
    app: &AppHandle,
    label: String,
    done: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    #[cfg(feature = "e2e-tests")]
    if crate::e2e::fault("fail_close") {
        return Err("Injected fixture native close failure".into());
    }
    let view = app
        .get_webview(&label)
        .ok_or("Native viewer is unavailable; closure is unconfirmed.")?;
    app.run_on_main_thread(move || {
        let parent = view.window();
        let result = view.close().map_err(|e| e.to_string());
        if result.is_ok()
            && !parent.label().starts_with("viewer-grid-")
            && parent.webviews().is_empty()
        {
            let _ = parent.destroy();
        }
        done(result);
    })
    .map_err(|e| e.to_string())
}

/// Run off the event thread; container construction must not block Windows UI.
pub fn switch(
    app: &AppHandle,
    target: ViewerLayout,
    revision: u64,
    labels: &[String],
    cancel: &Cancel,
) -> Result<MoveOutcome, MoveFailure> {
    if target == ViewerLayout::Grid {
        preflight(
            app,
            labels.len(),
            labels.iter().any(|label| label.starts_with("player-")),
        )?;
    }
    let mut targets = Vec::new();
    let mut old = Vec::new();
    let mut sources = Vec::new();
    for label in labels {
        let view = app
            .get_webview(label)
            .ok_or("Viewer disappeared before layout change.")?;
        let parent = view.window();
        let source = parent.label().to_owned();
        let bounds = bounds(&view)?;
        sources.push((parent, bounds));
        if !old.contains(&source) {
            old.push(source);
        }
    }
    let grid = if target == ViewerLayout::Grid && !labels.is_empty() {
        let label = format!("viewer-grid-{revision}");
        targets.push(container(app, &label, "MPD Viewer · Grid", true)?);
        Some(label)
    } else {
        None
    };
    let prepared = (|| -> Result<(), String> {
        if target == ViewerLayout::Standalone {
            for (index, label) in labels.iter().enumerate() {
                targets.push(container(
                    app,
                    &format!("viewer-standalone-{revision}-{index}"),
                    &format!("{label} · MPD Viewer"),
                    false,
                )?);
            }
        }
        Ok(())
    })();
    if let Err(error) = prepared {
        for window in targets {
            let _ = window.destroy();
        }
        return Err(format!("Layout preparation failed: {error}").into());
    }
    let bounds = if let Some(grid) = &grid {
        match arrange(app, grid, labels) {
            Ok(bounds) => bounds,
            Err(error) => {
                for window in targets {
                    let _ = window.destroy();
                }
                return Err(format!("Layout preparation failed: {error}").into());
            }
        }
    } else {
        vec![]
    };
    let labels = labels.to_vec();
    let cancel = cancel.clone();
    let app_clone = app.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let target_labels: Vec<_> = targets.iter().map(|w| w.label().to_owned()).collect();
    let containers = target_labels.clone();
    let dispatch = app.run_on_main_thread(move || {
        let mut moved = 0usize;
        let mut uncertain = false;
        let result = (|| -> Result<(), String> {
            for (index, label) in labels.iter().enumerate() {
                if cancel.load(Ordering::SeqCst) != 0 {
                    return Err("Layout change superseded.".into());
                }
                let view = app_clone
                    .get_webview(label)
                    .ok_or("Viewer disappeared during layout change.")?;
                let window = &targets[if target == ViewerLayout::Grid {
                    0
                } else {
                    index
                }];
                if let Err(error) = view.reparent(window) {
                    uncertain = true;
                    return Err(format!("Native viewer move failed: {error}"));
                }
                moved += 1;
                #[cfg(feature = "e2e-tests")]
                if crate::e2e::fault("fail_layout_after_move") {
                    return Err("Injected fixture failure after retained native move".into());
                }
                view.set_auto_resize(target == ViewerLayout::Standalone)
                    .map_err(|e| e.to_string())?;
                let rect = if target == ViewerLayout::Grid {
                    bounds[index]
                } else {
                    let size = window
                        .inner_size()
                        .map_err(|e| e.to_string())?
                        .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
                    Rect {
                        position: LogicalPosition::new(0.0, 0.0).into(),
                        size: size.into(),
                    }
                };
                set_bounds(&view, rect)?;
            }
            if cancel.load(Ordering::SeqCst) != 0 {
                return Err("Layout change superseded.".into());
            }
            for window in &targets {
                window.show().map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        let outcome = match result {
            Ok(()) => Ok(()),
            Err(message) => {
                // Only successfully moved children can be rolled back. A failed
                // reparent may already have dropped its runtime handle.
                if !uncertain {
                    for index in (0..moved).rev() {
                        let restored = (|| -> Result<(), ()> {
                            let view = app_clone.get_webview(&labels[index]).ok_or(())?;
                            view.reparent(&sources[index].0).map_err(|_| ())?;
                            set_bounds(&view, sources[index].1).map_err(|_| ())?;
                            view.set_auto_resize(
                                !sources[index].0.label().starts_with("viewer-grid-"),
                            )
                            .map_err(|_| ())
                        })();
                        if restored.is_err() {
                            uncertain = true;
                            break;
                        }
                    }
                }
                if uncertain {
                    // Publish cleanup intent before native destruction can
                    // deliver container events to the controller.
                    cancel.store(1, Ordering::SeqCst);
                    for label in old.iter().chain(&target_labels) {
                        if let Some(window) = app_clone.get_window(label) {
                            let _ = window.destroy();
                        }
                    }
                } else {
                    for window in &targets {
                        if window.webviews().is_empty() {
                            let _ = window.destroy();
                        }
                    }
                }
                Err(MoveFailure {
                    message,
                    source_intact: !uncertain,
                })
            }
        };
        let _ = tx.send(outcome.map(|_| {
            labels
                .into_iter()
                .zip(sources)
                .map(|(label, (window, bounds))| (label, window, bounds))
                .collect::<Vec<_>>()
        }));
    });
    if let Err(error) = dispatch {
        for label in &containers {
            if let Some(window) = app.get_window(label) {
                let _ = window.destroy();
            }
        }
        return Err(error.to_string().into());
    }
    let sources = rx.recv().map_err(|_| MoveFailure {
        message: "Native layout completion unavailable.".into(),
        source_intact: false,
    })??;
    Ok(MoveOutcome {
        grid,
        containers,
        sources,
    })
}

/// The controller may supersede completion after native work has finished.
/// Keep old empty sources until this settlement decision is made.
pub fn restore(
    app: &AppHandle,
    outcome: MoveOutcome,
    cancel: Cancel,
) -> Result<MoveOutcome, MoveFailure> {
    let app_clone = app.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let result = (|| -> Result<(), String> {
            for (label, window, bounds) in &outcome.sources {
                let view = app_clone
                    .get_webview(label)
                    .ok_or("Retained surface disappeared before rollback")?;
                view.reparent(window).map_err(|e| e.to_string())?;
                set_bounds(&view, *bounds)?;
                view.set_auto_resize(!window.label().starts_with("viewer-grid-"))
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        if result.is_err() {
            // Destruction events must observe Stop before releasing assignments.
            cancel.store(1, Ordering::SeqCst);
            for (_, window, _) in &outcome.sources {
                let _ = window.destroy();
            }
        }
        for label in &outcome.containers {
            if let Some(window) = app_clone.get_window(label) {
                let _ = window.destroy();
            }
        }
        let result = match result {
            Ok(()) => Err(MoveFailure {
                message: "Layout change superseded.".into(),
                source_intact: true,
            }),
            Err(message) => Err(MoveFailure {
                message,
                source_intact: false,
            }),
        };
        let _ = tx.send(result);
    })
    .map_err(|e| MoveFailure {
        message: e.to_string(),
        source_intact: false,
    })?;
    rx.recv().map_err(|_| MoveFailure {
        message: "Rollback completion unavailable".into(),
        source_intact: false,
    })?
}
