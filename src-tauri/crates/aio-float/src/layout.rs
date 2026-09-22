use crate::config::Config;
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutMode {
    Single,
    Horizontal,
    Vertical,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Split {
    pub ratio: f64,
    pub reversed: bool,
}

impl Default for Split {
    fn default() -> Self {
        Self { ratio: 0.5, reversed: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Pane {
    Requests,
    Providers,
}

#[derive(Debug, Serialize)]
pub struct Region {
    pub pane: Pane,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Region {
    fn new(pane: Pane, area: Rect) -> Self {
        Self { pane, x: area.x, y: area.y, width: area.width, height: area.height }
    }
    pub fn area(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

pub struct Geometry {
    pub header: Rect,
    pub separator: Rect,
    pub body: Rect,
    pub footer: Rect,
    pub divider: Rect,
    pub regions: Vec<Region>,
}

pub fn geometry(config: &Config, columns: u16, rows: u16, focus: Pane) -> Geometry {
    let header = Rect::new(0, 0, columns, rows.min(2));
    let separator = Rect::new(0, header.bottom(), columns, u16::from(rows > 2));
    let body = Rect::new(0, separator.bottom(), columns, rows.saturating_sub(4));
    let footer = Rect::new(0, rows.saturating_sub(1), columns, u16::from(rows > 3));
    let mut result = Geometry { header, separator, body, footer, divider: Rect::default(), regions: Vec::new() };
    if config.layout == LayoutMode::Single {
        result.regions.push(Region::new(focus, body));
        return result;
    }
    let horizontal = config.layout == LayoutMode::Horizontal;
    let split = if horizontal { &config.horizontal } else { &config.vertical };
    // Each pane needs a title and at least one content row.
    if columns == 0 || body.height < if horizontal { 2 } else { 5 } || (horizontal && columns < 3) {
        return result;
    }
    let total = if horizontal { body.width } else { body.height } - 1;
    let minimum = if horizontal { 1 } else { 2 };
    let first = split_size(total, split.ratio, minimum);
    let (a, divider, b) = if horizontal {
        (Rect::new(0, body.y, first, body.height), Rect::new(first, body.y, 1, body.height), Rect::new(first + 1, body.y, total - first, body.height))
    } else {
        (Rect::new(0, body.y, columns, first), Rect::new(0, body.y + first, columns, 1), Rect::new(0, body.y + first + 1, columns, total - first))
    };
    let (first_pane, second_pane) = if split.reversed { (Pane::Providers, Pane::Requests) } else { (Pane::Requests, Pane::Providers) };
    result.regions = vec![Region::new(first_pane, a), Region::new(second_pane, b)];
    result.divider = divider;
    result
}

fn split_size(total: u16, ratio: f64, minimum: u16) -> u16 {
    (f64::from(total) * ratio).round().clamp(f64::from(minimum), f64::from(total - minimum)) as u16
}

pub fn move_divider(config: &mut Config, columns: u16, rows: u16, key: &str) {
    let (split, total, minimum, delta) = match (config.layout, key) {
        (LayoutMode::Horizontal, "ArrowLeft" | "ArrowRight") if columns >= 3 && rows >= 6 =>
            (&mut config.horizontal, columns - 1, 1, if key == "ArrowLeft" { -1 } else { 1 }),
        (LayoutMode::Vertical, "ArrowUp" | "ArrowDown") if rows >= 9 =>
            (&mut config.vertical, rows - 5, 2, if key == "ArrowUp" { -1 } else { 1 }),
        _ => return,
    };
    let current = split_size(total, split.ratio, minimum);
    let next = current.saturating_add_signed(delta).clamp(minimum, total - minimum);
    if next != current {
        split.ratio = f64::from(next) / f64::from(total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divider_moves_by_one_cell_and_preserves_independent_ratios() {
        let mut config = Config { layout: LayoutMode::Horizontal, ..Config::default() };
        let before = geometry(&config, 80, 40, Pane::Requests);
        move_divider(&mut config, 80, 40, "ArrowRight");
        assert_eq!(geometry(&config, 80, 40, Pane::Requests).divider.x, before.divider.x + 1);
        let horizontal = config.horizontal.ratio;
        move_divider(&mut config, 80, 40, "ArrowDown");
        assert_eq!(config.horizontal.ratio, horizontal);
        config.layout = LayoutMode::Vertical;
        let before = geometry(&config, 80, 40, Pane::Requests);
        move_divider(&mut config, 80, 40, "ArrowUp");
        assert_eq!(geometry(&config, 80, 40, Pane::Requests).divider.y, before.divider.y - 1);
        config.layout = LayoutMode::Horizontal;
        assert_eq!(config.horizontal.ratio, horizontal);
        let resized = geometry(&config, 160, 40, Pane::Requests);
        assert!((f64::from(resized.regions[0].width) / 159.0 - horizontal).abs() <= 1.0 / 159.0);
        config.horizontal.reversed = true;
        let swapped = geometry(&config, 160, 40, Pane::Requests);
        assert_eq!(resized.divider, swapped.divider);
        assert_eq!(swapped.regions[0].pane, Pane::Providers);
    }

    #[test]
    fn tiny_grids_and_extreme_ratios_do_not_overlap_or_hide_a_pane() {
        for mode in [LayoutMode::Horizontal, LayoutMode::Vertical] {
            for columns in [1, 2, 3, 8, 80] {
                for rows in [1, 4, 6, 9, 40] {
                    for ratio in [0.0, 0.5, 1.0] {
                        let config = Config { layout: mode, horizontal: Split { ratio, reversed: false }, vertical: Split { ratio, reversed: false }, ..Config::default() };
                        let g = geometry(&config, columns, rows, Pane::Requests);
                        assert!(g.regions.is_empty() || g.regions.len() == 2);
                        for region in &g.regions {
                            assert!(region.width > 0 && region.height >= 2);
                            assert!(region.x + region.width <= columns);
                            assert!(region.y + region.height <= rows);
                            assert!(region.area().intersection(g.divider).is_empty());
                        }
                    }
                }
            }
        }
    }
}
