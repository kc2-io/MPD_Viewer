//! Pure logical-pixel geometry. Presentation never truncates controller selection.
use tauri::{LogicalPosition, LogicalSize, Rect};

// Native parent background shows through this logical-pixel divider.
pub const DIVIDER: f64 = 2.0;
pub const MIN_WIDTH: f64 = 430.0;
pub const MIN_HEIGHT: f64 = 480.0;

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
