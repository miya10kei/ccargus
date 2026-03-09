use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Center a rectangle with percentage-based width and fixed pixel height.
pub fn centered_rect_fixed_height(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

/// Center a rectangle with percentage-based width and height.
pub fn centered_rect_percent(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_fixed_height_produces_valid_area() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect_fixed_height(50, 5, area);
        assert!(popup.width > 0);
        assert!(popup.height > 0);
        assert!(popup.x > 0);
        assert!(popup.y > 0);
    }

    #[test]
    fn centered_rect_percent_produces_valid_area() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect_percent(80, 80, area);
        assert!(popup.width > 0);
        assert!(popup.height > 0);
        assert!(popup.x > 0);
        assert!(popup.y > 0);
    }
}
