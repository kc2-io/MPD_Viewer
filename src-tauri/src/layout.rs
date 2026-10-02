//! Pure logical-pixel geometry. Presentation never truncates controller selection.
use tauri::{LogicalPosition, LogicalSize, Rect};

// Native parent background shows through this logical-pixel divider.
pub const DIVIDER: f64 = 2.0;
pub const MIN_WIDTH: f64 = 430.0;
pub const MIN_HEIGHT: f64 = 480.0;

#[derive(Clone, Copy, Debug)]
pub struct PhysicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn best_work_area(window: PhysicalRect, areas: &[PhysicalRect]) -> Option<usize> {
    areas
        .iter()
        .enumerate()
        .filter_map(|(i, area)| {
            let width = (window.x + window.width).min(area.x + area.width) - window.x.max(area.x);
            let height =
                (window.y + window.height).min(area.y + area.height) - window.y.max(area.y);
            (width > 0.0 && height > 0.0).then_some((i, width * height))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

pub fn reachable(window: PhysicalRect, areas: &[PhysicalRect]) -> bool {
    let tolerance = 16.0; // Physical pixels: Windows invisible resize borders.
    let expanded: Vec<_> = areas
        .iter()
        .map(|a| PhysicalRect {
            x: a.x - tolerance,
            y: a.y - tolerance,
            width: a.width + tolerance * 2.0,
            height: a.height + tolerance * 2.0,
        })
        .collect();
    best_work_area(window, &expanded).is_some()
}

pub fn grid_fits(
    count: usize,
    client: (f64, f64),
    available: (f64, f64),
    managed: bool,
    minimum: (f64, f64),
) -> Result<(), String> {
    match cells_with_min(count, client.0, client.1, minimum.0, minimum.1) {
        Ok(_) => Ok(()),
        Err(error) if managed => Err(error),
        Err(_) => cells_with_min(count, available.0, available.1, minimum.0, minimum.1).map(|_| ()),
    }
}

#[cfg(test)]
pub fn cells(count: usize, width: f64, height: f64) -> Result<Vec<Rect>, String> {
    cells_with_min(count, width, height, MIN_WIDTH, MIN_HEIGHT)
}

pub fn cells_with_min(
    count: usize,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
) -> Result<Vec<Rect>, String> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("Grid display dimensions are unavailable.".into());
    }
    if count == 0 {
        return Ok(vec![]);
    }
    if count > 1000 {
        return Err("Grid selection exceeds the viewer safety bound.".into());
    }
    let mut best = None;
    for columns in 1..=count {
        let rows = count.div_ceil(columns);
        let w = (width - DIVIDER * (columns - 1) as f64) / columns as f64;
        let h = (height - DIVIDER * (rows - 1) as f64) / rows as f64;
        if w < min_width || h < min_height {
            continue;
        }
        let score = (w / min_width).min(h / min_height);
        if best.is_none_or(|(_, _, previous)| score > previous) {
            best = Some((columns, rows, score));
        }
    }
    let (columns, rows, _) = best.ok_or_else(|| format!("Grid cannot fit {count} selected viewers at the minimum {min_width:.0} × {min_height:.0} cell size. Resize the window or choose Standalone."))?;
    let w = (width - DIVIDER * (columns - 1) as f64) / columns as f64;
    let h = (height - DIVIDER * (rows - 1) as f64) / rows as f64;
    Ok((0..count)
        .map(|i| Rect {
            position: LogicalPosition::new(
                (i % columns) as f64 * (w + DIVIDER),
                (i / columns) as f64 * (h + DIVIDER),
            )
            .into(),
            size: LogicalSize::new(w, h).into(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_client_is_authority_even_with_shorter_decorations() {
        assert!(grid_fits(2, (862.0, 480.0), (862.0, 450.0), true, (430.0, 480.0)).is_ok());
        assert!(grid_fits(2, (862.0, 450.0), (862.0, 480.0), true, (430.0, 480.0)).is_err());
        assert!(grid_fits(2, (862.0, 450.0), (862.0, 480.0), false, (430.0, 480.0)).is_ok());
    }
    #[test]
    fn placement_respects_other_monitors_partial_overlap_and_decorations() {
        let areas = [
            PhysicalRect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            PhysicalRect {
                x: 1920.0,
                y: -200.0,
                width: 2560.0,
                height: 1440.0,
            },
        ];
        let mut grid = PhysicalRect {
            x: 2100.0,
            y: -8.0,
            width: 1400.0,
            height: 900.0,
        };
        assert_eq!(best_work_area(grid, &areas), Some(1));
        assert!(reachable(grid, &areas));
        grid.x = -900.0;
        assert!(reachable(grid, &areas));
        grid.x = -1410.0; // Ten pixels outside: decoration tolerance.
        assert!(reachable(grid, &areas));
        grid.x = 5000.0;
        assert!(!reachable(grid, &areas));
        assert!(!reachable(grid, &[]));
    }
    #[test]
    fn geometry_preserves_count_minimum_and_bounds() {
        for count in [1, 2, 3, 4, 6, 12] {
            let result = cells(count, 2600.0, 1450.0).unwrap();
            assert_eq!(result.len(), count);
            for rect in result {
                let pos = rect.position.to_logical::<f64>(1.0);
                let size = rect.size.to_logical::<f64>(1.0);
                assert!(size.width >= MIN_WIDTH && size.height >= MIN_HEIGHT);
                assert!(
                    pos.x + size.width <= 2600.0 + 0.01 && pos.y + size.height <= 1450.0 + 0.01
                );
            }
        }
    }
    #[test]
    fn dividers_separate_columns_and_rows_without_shrinking_minimums() {
        let result = cells(4, 862.0, 962.0).unwrap();
        for (left, right) in [(0, 1), (2, 3)] {
            let a = result[left].position.to_logical::<f64>(1.0);
            let b = result[right].position.to_logical::<f64>(1.0);
            let size = result[left].size.to_logical::<f64>(1.0);
            assert_eq!(b.x - a.x - size.width, DIVIDER);
        }
        assert_eq!(
            result[2].position.to_logical::<f64>(1.0).y
                - result[0].size.to_logical::<f64>(1.0).height,
            DIVIDER
        );
        assert_eq!(
            result[0].size.to_logical::<f64>(1.0),
            LogicalSize::new(MIN_WIDTH, MIN_HEIGHT)
        );
    }
    #[test]
    fn no_fit_does_not_return_a_partial_selection() {
        assert!(cells(7, 1290.0, 960.0).is_err());
        assert!(cells(6, 1290.0, 960.0).is_err());
        assert_eq!(cells(6, 1294.0, 962.0).unwrap().len(), 6);
        assert!(cells(1, f64::NAN, 960.0).is_err());
        assert!(cells(1000, 1290.0, 960.0).is_err());
        assert!(cells(1, 1290.0, 960.0).is_ok());
    }
}
