//! Native containers and retained surfaces, independent of selection policy.
use crate::{layout, model::ViewerLayout};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
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

#[derive(Debug)]
pub enum GridError {
    NoFit(String, std::time::Instant),
    Observation(String),
}
impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFit(s, _) | Self::Observation(s) => f.write_str(s),
        }
    }
}
impl From<String> for GridError {
    fn from(s: String) -> Self {
        Self::Observation(s)
    }
}
impl From<GridError> for String {
    fn from(e: GridError) -> Self {
        e.to_string()
    }
}
impl From<GridError> for MoveFailure {
    fn from(e: GridError) -> Self {
        e.to_string().into()
    }
}
#[derive(Clone, Debug)]
struct WorkArea {
    rect: layout::PhysicalRect,
    scale: f64,
}
impl WorkArea {
    fn size(&self) -> (f64, f64) {
        (
            self.rect.width / self.scale,
            (self.rect.height / self.scale - 40.0).max(1.0),
        )
    }
}
#[derive(Clone, Debug)]
struct GridWindow {
    label: String,
    outer: layout::PhysicalRect,
    inner: LogicalSize<f64>,
    managed: bool,
    area: usize,
}
#[derive(Clone, Debug)]
pub struct DisplayObservation {
    pub observed: std::time::Instant,
    areas: Vec<WorkArea>,
    preferred: usize,
    grid: Option<GridWindow>,
}
impl DisplayObservation {
    pub fn fresh_for(&self, grid: Option<&str>) -> bool {
        self.observed.elapsed() <= std::time::Duration::from_secs(2)
            && self.grid.as_ref().map(|g| g.label.as_str()) == grid
    }
    #[cfg(feature = "e2e-tests")]
    fn diagnose_no_fit(
        &self,
        phase: &'static str,
        count: usize,
        labels: &[String],
        minimum: (f64, f64),
        error: &str,
    ) {
        let area = self.grid.as_ref().map_or(self.preferred, |g| g.area);
        let areas=self.areas.iter().take(8).map(|a|serde_json::json!({
            "physical_work_area":{"x":a.rect.x,"y":a.rect.y,"width":a.rect.width,"height":a.rect.height},
            "scale":a.scale,"usable_logical":a.size()})).collect::<Vec<_>>();
        let grid=self.grid.as_ref().map(|g|serde_json::json!({"label":g.label,"logical_inner":g.inner,
            "physical_outer":{"x":g.outer.x,"y":g.outer.y,"width":g.outer.width,"height":g.outer.height},"managed":g.managed}));
        crate::grid_probe::diagnostic(
            crate::grid_probe::diagnostic_stage(),
            serde_json::json!({
            "no_fit_phase":phase,"error":error,"count":count,"labels":labels.iter().take(8).collect::<Vec<_>>(),
            "labels_total":labels.len(),"minimum_logical":minimum,"snapshot_age_ms":self.observed.elapsed().as_millis(),
            "grid":grid,"area_index":area,"preferred_area_index":self.preferred,"areas":areas,"areas_total":self.areas.len()}),
        );
    }
    pub fn preflight(&self, count: usize, embedded: bool) -> Result<(), GridError> {
        let area = &self.areas[self.grid.as_ref().map_or(self.preferred, |g| g.area)];
        let (w, h) = area.size();
        let minimum = if embedded {
            (800.0, 540.0)
        } else {
            (layout::MIN_WIDTH, layout::MIN_HEIGHT)
        };
        let (client, managed) = self.grid.as_ref().map_or(((w, h), false), |g| {
            ((g.inner.width, g.inner.height), g.managed)
        });
        layout::grid_fits(count, client, (w, h), managed, minimum).map_err(|e| {
            #[cfg(feature = "e2e-tests")]
            self.diagnose_no_fit("preflight", count, &[], minimum, &e);
            GridError::NoFit(e, self.observed)
        })
    }
}
fn observe(app: &AppHandle, grid: Option<&str>) -> Result<DisplayObservation, GridError> {
    #[cfg(feature = "e2e-tests")]
    {
        if crate::e2e::fault("slow_display_observation") {
            let _ = std::fs::write(
                crate::e2e::root().join("display-observation-started"),
                b"fixture",
            );
            std::thread::sleep(std::time::Duration::from_secs(6));
        }
        if crate::e2e::fault("fail_display_observation") {
            return Err(GridError::Observation(
                "Injected display observation failure".into(),
            ));
        }
    }
    let app_clone = app.clone();
    let grid = grid.map(str::to_owned);
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let result = (|| -> Result<DisplayObservation, String> {
            // Locked runtime monitor conversion calls GTK/AppKit. Capture all
            // native geometry on this event thread, never on the controller.
            let monitors = app_clone.available_monitors().map_err(|e| e.to_string())?;
            let areas: Vec<_> = monitors
                .iter()
                .map(|m| {
                    let a = m.work_area();
                    WorkArea {
                        rect: layout::PhysicalRect {
                            x: a.position.x as f64,
                            y: a.position.y as f64,
                            width: a.size.width as f64,
                            height: a.size.height as f64,
                        },
                        scale: m.scale_factor(),
                    }
                })
                .collect();
            if areas.is_empty()
                || areas.iter().any(|a| {
                    !a.scale.is_finite()
                        || a.scale <= 0.0
                        || a.rect.width <= 0.0
                        || a.rect.height <= 0.0
                })
            {
                return Err("Display work areas are unavailable or invalid.".into());
            }
            let preferred_monitor = app_clone
                .get_window("main")
                .and_then(|w| w.current_monitor().ok().flatten())
                .or_else(|| app_clone.primary_monitor().ok().flatten());
            let preferred = preferred_monitor
                .and_then(|m| monitors.iter().position(|a| a.position() == m.position()))
                .unwrap_or(0);
            let grid = grid
                .map(|label| -> Result<GridWindow, String> {
                    let w = app_clone
                        .get_window(&label)
                        .ok_or("Grid window disappeared during observation.")?;
                    let pos = w.outer_position().map_err(|e| e.to_string())?;
                    let outer_size = w.outer_size().map_err(|e| e.to_string())?;
                    let scale = w.scale_factor().map_err(|e| e.to_string())?;
                    if !scale.is_finite() || scale <= 0.0 {
                        return Err("Invalid grid display scale.".into());
                    }
                    let inner = w
                        .inner_size()
                        .map_err(|e| e.to_string())?
                        .to_logical::<f64>(scale);
                    if inner.width <= 0.0 || inner.height <= 0.0 {
                        return Err("Grid client dimensions are unavailable.".into());
                    }
                    let outer = layout::PhysicalRect {
                        x: pos.x as f64,
                        y: pos.y as f64,
                        width: outer_size.width as f64,
                        height: outer_size.height as f64,
                    };
                    let area = w
                        .current_monitor()
                        .map_err(|e| e.to_string())?
                        .and_then(|m| monitors.iter().position(|a| a.position() == m.position()))
                        .unwrap_or_else(|| {
                            layout::best_work_area(
                                outer,
                                &areas.iter().map(|a| a.rect).collect::<Vec<_>>(),
                            )
                            .unwrap_or(preferred)
                        });
                    let managed = w.is_maximized().map_err(|e| e.to_string())?
                        || w.is_fullscreen().map_err(|e| e.to_string())?;
                    Ok(GridWindow {
                        label,
                        outer,
                        inner,
                        managed,
                        area,
                    })
                })
                .transpose()?;
            Ok(DisplayObservation {
                observed: std::time::Instant::now(),
                areas,
                preferred,
                grid,
            })
        })();
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| GridError::Observation("Display observation timed out".into()))?
        .map_err(GridError::Observation)
}
fn screen(app: &AppHandle) -> Result<(f64, f64, tauri::PhysicalPosition<i32>, f64), String> {
    let snapshot = observe(app, None)?;
    let area = &snapshot.areas[snapshot.preferred];
    let (w, h) = area.size();
    Ok((
        w,
        h,
        tauri::PhysicalPosition::new(area.rect.x as i32, area.rect.y as i32),
        area.scale,
    ))
}
pub fn preflight(app: &AppHandle, count: usize, embedded: bool) -> Result<(), GridError> {
    if count == 0 {
        return Ok(());
    }
    observe(app, None)?.preflight(count, embedded)
}

pub fn grid_work(
    app: &AppHandle,
    grid: Option<&str>,
    labels: &[String],
    desired: usize,
    embedded: bool,
    cancel: &Cancel,
) -> Result<DisplayObservation, GridError> {
    let snapshot = observe(app, grid)?;
    if cancel.load(Ordering::SeqCst) != 0 {
        return Err(GridError::Observation("Grid observation cancelled".into()));
    }
    if desired > labels.len() || grid.is_none() {
        snapshot.preflight(desired, embedded)?;
    }
    if let Some(grid) = grid {
        if !labels.is_empty() {
            arrange_observed(app, grid, labels, &snapshot, cancel)?;
        }
    }
    Ok(snapshot)
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
    create_container(app, label, title, grid, None)
}
fn create_container(
    app: &AppHandle,
    label: &str,
    title: &str,
    grid: bool,
    cancel: Option<&Cancel>,
) -> Result<Window, String> {
    let active = || {
        if cancel.is_some_and(|c| c.load(Ordering::SeqCst) != 0) {
            Err("Grid preparation cancelled".to_owned())
        } else {
            Ok(())
        }
    };
    active()?;
    #[cfg(feature = "e2e-tests")]
    if crate::e2e::fault("fail_layout_prepare") {
        return Err("Injected fixture container creation failure".into());
    }
    let (w, h, pos, _scale) = screen(app)?;
    active()?;
    let mut builder = tauri::window::WindowBuilder::new(app, label);
    if grid {
        let theme = app
            .get_window("main")
            .ok_or("Manager is unavailable")?
            .theme()
            .map_err(|e| e.to_string())?;
        builder = builder.background_color(divider_color(theme));
    }
    active()?;
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
    // Builder creation is not cancellable inside Tauri. A hidden container
    // returned after cancellation is still owned and retired before any child.
    let configured = active()
        .and_then(|()| {
            let configure = move |window: &Window| -> Result<(), String> {
                window
                    .set_size(LogicalSize::new(
                        if grid { w.min(1440.0) } else { w.min(1180.0) },
                        if grid { h.min(960.0) } else { h.min(720.0) },
                    ))
                    .map_err(|e| e.to_string())?;
                window.set_position(pos).map_err(|e| e.to_string())?;
                Ok(())
            };
            if let Some(cancel) = cancel {
                mutate_window(&window, cancel, configure).map_err(String::from)
            } else {
                configure(&window)
            }
        })
        .and_then(|()| active());
    if let Err(error) = configured {
        let _ = window.destroy();
        return Err(error);
    }
    Ok(window)
}

pub struct PreparedGridOpen {
    pub window: Window,
    pub bounds: Rect,
    pub observed: std::time::Instant,
}

/// Prepare a cell without starting playback. The controller owns `grid` before
/// dispatch, including hidden containers that materialize after cancellation.
pub fn prepare_grid_open(
    app: &AppHandle,
    grid: &str,
    new_grid: bool,
    labels: &[String],
    desired: usize,
    embedded: bool,
    cancel: &Cancel,
) -> Result<PreparedGridOpen, GridError> {
    #[cfg(feature = "e2e-tests")]
    grid_open_barrier("delay_grid_open", "grid-open-waiting")?;
    if cancel.load(Ordering::SeqCst) != 0 {
        return Err(GridError::Observation("Grid preparation cancelled".into()));
    }
    if new_grid {
        create_container(app, grid, "MPD Viewer · Grid", true, Some(cancel))?;
    }
    #[cfg(feature = "e2e-tests")]
    grid_open_barrier("delay_grid_open_created", "grid-open-created-waiting")?;
    if cancel.load(Ordering::SeqCst) != 0 {
        return Err(GridError::Observation("Grid preparation cancelled".into()));
    }
    let snapshot = observe(app, Some(grid))?;
    if cancel.load(Ordering::SeqCst) != 0 {
        return Err(GridError::Observation("Grid preparation cancelled".into()));
    }
    // Admission remains for the entire Rust-selected set, not just the next cell.
    snapshot.preflight(desired, embedded)?;
    let bounds = arrange_observed(app, grid, labels, &snapshot, cancel)?;
    let window = app
        .get_window(grid)
        .ok_or_else(|| "Grid disappeared.".to_owned())?;
    Ok(PreparedGridOpen {
        window,
        bounds: *bounds
            .last()
            .ok_or_else(|| "Grid preparation needs a viewer.".to_owned())?,
        observed: snapshot.observed,
    })
}

#[cfg(feature = "e2e-tests")]
fn grid_open_barrier(fault: &str, marker: &str) -> Result<(), GridError> {
    if crate::e2e::fault(fault) {
        let root = crate::e2e::root();
        std::fs::write(root.join(marker), b"fixture").map_err(|e| e.to_string())?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !root.join("grid-open-release").exists() {
            if std::time::Instant::now() >= deadline {
                return Err(GridError::Observation(
                    "Grid preparation fixture barrier timed out".into(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    Ok(())
}

pub fn arrange(app: &AppHandle, grid: &str, labels: &[String]) -> Result<Vec<Rect>, GridError> {
    let snapshot = observe(app, Some(grid))?;
    let cancel = Arc::new(AtomicU8::new(0));
    let result = arrange_observed(app, grid, labels, &snapshot, &cancel);
    cancel.store(1, Ordering::SeqCst);
    result
}
fn mutate_window(
    window: &Window,
    cancel: &Cancel,
    mutation: impl FnOnce(&Window) -> Result<(), String> + Send + 'static,
) -> Result<(), GridError> {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    let cancel = cancel.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let dispatch = window.app_handle().clone();
    let callback = move || {
        let result = if cancel.load(Ordering::SeqCst) != 0 {
            #[cfg(feature = "e2e-tests")]
            if crate::e2e::fault("delay_grid_mutation")
                || crate::e2e::root().join("grid-mutation-delayed").exists()
            {
                let _ =
                    std::fs::write(crate::e2e::root().join("grid-mutation-skipped"), b"fixture");
            }
            Err("Grid arrangement cancelled".into())
        } else if let Some(window) = app.get_window(&label) {
            mutation(&window)
        } else {
            Err("Grid window is no longer available".into())
        };
        let _ = tx.send(result);
    };
    #[cfg(feature = "e2e-tests")]
    let delayed = crate::e2e::fault("delay_grid_mutation");
    #[cfg(not(feature = "e2e-tests"))]
    let delayed = false;
    if delayed {
        #[cfg(feature = "e2e-tests")]
        let _ = std::fs::write(crate::e2e::root().join("grid-mutation-delayed"), b"fixture");
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(6));
            let _ = dispatch.run_on_main_thread(callback);
        });
    } else {
        dispatch
            .run_on_main_thread(callback)
            .map_err(|e| e.to_string())?;
    }
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| GridError::Observation("Grid mutation timed out".into()))?
        .map_err(GridError::Observation)
}
fn arrange_observed(
    app: &AppHandle,
    grid: &str,
    labels: &[String],
    snapshot: &DisplayObservation,
    cancel: &Cancel,
) -> Result<Vec<Rect>, GridError> {
    if !snapshot.fresh_for(Some(grid)) {
        return Err(GridError::Observation(
            "Grid observation expired or changed container".into(),
        ));
    }
    let window = app
        .get_window(grid)
        .ok_or_else(|| GridError::Observation("Grid window is no longer available".into()))?;
    let observed = snapshot.grid.as_ref().unwrap();
    let mut size = observed.inner;
    let area = &snapshot.areas[observed.area];
    let (available_w, available_h) = area.size();
    // Partial overlap, edge-snapping and placement on another monitor are user
    // choices. Recover only a wholly unreachable normal window.
    let recover = !observed.managed
        && !layout::reachable(
            observed.outer,
            &snapshot.areas.iter().map(|a| a.rect).collect::<Vec<_>>(),
        );
    if recover {
        size = LogicalSize::new(size.width.min(available_w), size.height.min(available_h));
        let pos = tauri::PhysicalPosition::new(area.rect.x as i32, area.rect.y as i32);
        let areas = snapshot.areas.iter().map(|a| a.rect).collect::<Vec<_>>();
        mutate_window(&window, cancel, move |w| {
            ensure_normal(w)?;
            let current = w.outer_position().map_err(|e| e.to_string())?;
            let outer = w.outer_size().map_err(|e| e.to_string())?;
            if layout::reachable(
                layout::PhysicalRect {
                    x: current.x as f64,
                    y: current.y as f64,
                    width: outer.width as f64,
                    height: outer.height as f64,
                },
                &areas,
            ) {
                return Err("Grid placement changed during recovery".into());
            }
            w.set_min_size(None::<LogicalSize<f64>>)
                .map_err(|e| e.to_string())?;
            w.set_position(pos).map_err(|e| e.to_string())?;
            w.set_size(size).map_err(|e| e.to_string())
        })?;
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
        Err(error) if observed.managed => {
            #[cfg(feature = "e2e-tests")]
            snapshot.diagnose_no_fit(
                "managed-arrangement",
                labels.len(),
                labels,
                (min_width, min_height),
                &error,
            );
            return Err(GridError::NoFit(error, snapshot.observed));
        }
        Err(_) => {
            let bounds = layout::cells_with_min(
                labels.len(),
                available_w,
                available_h,
                min_width,
                min_height,
            )
            .map_err(|e| {
                #[cfg(feature = "e2e-tests")]
                snapshot.diagnose_no_fit(
                    "normal-growth",
                    labels.len(),
                    labels,
                    (min_width, min_height),
                    &e,
                );
                GridError::NoFit(e, snapshot.observed)
            })?;
            size = LogicalSize::new(available_w, available_h);
            // Grow on the grid's own monitor without moving a reachable window.
            mutate_window(&window, cancel, move |w| {
                ensure_normal(w)?;
                w.set_size(size).map_err(|e| e.to_string())
            })?;
            bounds
        }
    };
    if let Some(first) = bounds.first().filter(|_| !observed.managed) {
        let cell = first.size.to_logical::<f64>(1.0);
        let columns = ((size.width + layout::DIVIDER) / (cell.width + layout::DIVIDER)).round();
        let rows = ((size.height + layout::DIVIDER) / (cell.height + layout::DIVIDER)).round();
        mutate_window(&window, cancel, move |w| {
            ensure_normal(w)?;
            w.set_min_size(Some(LogicalSize::new(
                columns * min_width + (columns - 1.0) * layout::DIVIDER,
                rows * min_height + (rows - 1.0) * layout::DIVIDER,
            )))
            .map_err(|e| e.to_string())
        })?;
    }
    for (label, bounds) in labels.iter().zip(&bounds) {
        if cancel.load(Ordering::SeqCst) != 0 {
            return Err(GridError::Observation("Grid arrangement cancelled".into()));
        }
        if let Some(webview) = app
            .get_webview(label)
            .filter(|v| v.window().label() == grid)
        {
            set_bounds_checked(&webview, *bounds, Some(cancel.clone()))?;
        }
    }
    let title = format!("MPD Viewer · {} viewers", labels.len());
    mutate_window(&window, cancel, move |w| {
        w.set_title(&title).map_err(|e| e.to_string())
    })?;
    Ok(bounds)
}

fn ensure_normal(window: &Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())?
        || window.is_fullscreen().map_err(|e| e.to_string())?
    {
        Err("Grid management state changed during observation".into())
    } else {
        Ok(())
    }
}

/// Locked Linux Wry uses a GtkBox and ignores cell coordinates there. Keep
/// Tauri's native ownership, but place grid widgets in a native GtkFixed.
/// This changes neither browser security nor document content.
pub fn set_bounds(view: &tauri::Webview, bounds: Rect) -> Result<(), String> {
    set_bounds_checked(view, bounds, None)
}
fn set_bounds_checked(
    view: &tauri::Webview,
    bounds: Rect,
    cancel: Option<Cancel>,
) -> Result<(), String> {
    if let Some(cancel) = &cancel {
        let view = view.clone();
        let window = view.window();
        let parent = window.label().to_owned();
        mutate_window(&window, cancel, move |_| {
            if view.window().label() != parent {
                return Err("Grid child changed parent".into());
            }
            view.set_bounds(bounds).map_err(|e| e.to_string())
        })?;
    } else {
        view.set_bounds(bounds).map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let window = view.window();
        let (tx, rx) = std::sync::mpsc::channel();
        view.with_webview(move |platform| {
            use gtk::prelude::*;
            let result = (|| -> Result<(), String> {
                if cancel
                    .as_ref()
                    .is_some_and(|c| c.load(Ordering::SeqCst) != 0)
                {
                    return Err("Grid bounds cancelled".into());
                }
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
                Foundation::{ERROR_SUCCESS, GetLastError, HWND, POINT, RECT, SetLastError},
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
