//! The cursor while this computer's input is captured and sent to a target
//! (R-INPUT-2, R-LAYOUT-5, R-LAYOUT-6).
//!
//! While captured, the local cursor is held still and only motion deltas
//! arrive. This keeps a virtual cursor on the target's display in target
//! pixels. Pushing it through the one edge that links back to the controller
//! ends the capture; every other edge is a wall.

use q2_layout::Edge;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitRule {
    /// Beside: exiting through a single edge leads back to the neighbour.
    Single(Edge),
    /// Windowed Inside: hitting any of the 4 borders exits back to the host (R-LAYOUT-6).
    AllBorders,
    /// Fullscreen Inside: exits through any linked edge of the host monitor (R-LAYOUT-6).
    LinkedEdges {
        left: bool,
        right: bool,
        top: bool,
        bottom: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Still on the target display, at this pixel.
    Inside { x: i32, y: i32 },
    /// Left through an exit edge. `along` is the position along that edge
    /// (the y pixel for a left or right edge, the x pixel for top or bottom),
    /// already clamped to the display.
    Exit { along: i32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Capture {
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    rule: ExitRule,
    last_exit: Option<Edge>,
}

impl Capture {
    /// Start a capture on a display of `size_px`, with the cursor at `entry_px`
    /// and `exit` as the edge that leads back.
    pub fn enter(size_px: (i32, i32), entry_px: (i32, i32), exit: Edge) -> Self {
        Self::enter_with_rule(size_px, entry_px, ExitRule::Single(exit))
    }

    /// Start a capture with a custom exit rule (for Inside windowed or fullscreen).
    pub fn enter_with_rule(size_px: (i32, i32), entry_px: (i32, i32), rule: ExitRule) -> Self {
        let width = size_px.0.max(1);
        let height = size_px.1.max(1);
        Self {
            width,
            height,
            x: entry_px.0.clamp(0, width - 1),
            y: entry_px.1.clamp(0, height - 1),
            rule,
            last_exit: None,
        }
    }

    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn exit_edge(&self) -> Edge {
        match self.rule {
            ExitRule::Single(edge) => edge,
            _ => self.last_exit.unwrap_or(Edge::Left),
        }
    }

    pub fn last_exit_edge(&self) -> Option<Edge> {
        self.last_exit
    }

    /// Apply a motion delta from the local mouse.
    pub fn motion(&mut self, dx: i32, dy: i32) -> Motion {
        let nx = self.x.saturating_add(dx);
        let ny = self.y.saturating_add(dy);
        let max_x = self.width - 1;
        let max_y = self.height - 1;

        let is_exit = |edge: Edge, rule: ExitRule| -> bool {
            match rule {
                ExitRule::Single(e) => e == edge,
                ExitRule::AllBorders => true,
                ExitRule::LinkedEdges {
                    left,
                    right,
                    top,
                    bottom,
                } => match edge {
                    Edge::Left => left,
                    Edge::Right => right,
                    Edge::Top => top,
                    Edge::Bottom => bottom,
                },
            }
        };

        if nx < 0 && is_exit(Edge::Left, self.rule) {
            self.x = 0;
            self.y = ny.clamp(0, max_y);
            self.last_exit = Some(Edge::Left);
            return Motion::Exit { along: self.y };
        }
        if nx > max_x && is_exit(Edge::Right, self.rule) {
            self.x = max_x;
            self.y = ny.clamp(0, max_y);
            self.last_exit = Some(Edge::Right);
            return Motion::Exit { along: self.y };
        }
        if ny < 0 && is_exit(Edge::Top, self.rule) {
            self.x = nx.clamp(0, max_x);
            self.y = 0;
            self.last_exit = Some(Edge::Top);
            return Motion::Exit { along: self.x };
        }
        if ny > max_y && is_exit(Edge::Bottom, self.rule) {
            self.x = nx.clamp(0, max_x);
            self.y = max_y;
            self.last_exit = Some(Edge::Bottom);
            return Motion::Exit { along: self.x };
        }

        self.x = nx.clamp(0, max_x);
        self.y = ny.clamp(0, max_y);
        Motion::Inside {
            x: self.x,
            y: self.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_inside_and_tracks_the_pixel() {
        let mut c = Capture::enter((1920, 1080), (0, 500), Edge::Left);
        assert_eq!(c.motion(30, -10), Motion::Inside { x: 30, y: 490 });
        assert_eq!(c.motion(5, 5), Motion::Inside { x: 35, y: 495 });
        assert_eq!(c.position(), (35, 495));
    }

    #[test]
    fn pushing_through_the_exit_edge_leaves() {
        let mut c = Capture::enter((1920, 1080), (10, 700), Edge::Left);
        assert_eq!(c.motion(-5, 0), Motion::Inside { x: 5, y: 700 });
        assert_eq!(c.motion(-20, 3), Motion::Exit { along: 703 });
    }

    #[test]
    fn other_edges_are_walls() {
        let mut c = Capture::enter((1920, 1080), (100, 100), Edge::Left);
        assert_eq!(c.motion(0, -500), Motion::Inside { x: 100, y: 0 });
        assert_eq!(c.motion(5000, 5000), Motion::Inside { x: 1919, y: 1079 });
    }

    #[test]
    fn exit_position_is_clamped_to_the_display() {
        let mut c = Capture::enter((1920, 1080), (0, 1070), Edge::Left);
        assert_eq!(c.motion(-1, 100), Motion::Exit { along: 1079 });
    }

    #[test]
    fn right_top_and_bottom_exits() {
        let mut c = Capture::enter((800, 600), (790, 300), Edge::Right);
        assert_eq!(c.motion(20, 0), Motion::Exit { along: 300 });
        let mut c = Capture::enter((800, 600), (400, 5), Edge::Top);
        assert_eq!(c.motion(2, -9), Motion::Exit { along: 402 });
        let mut c = Capture::enter((800, 600), (400, 595), Edge::Bottom);
        assert_eq!(c.motion(-3, 9), Motion::Exit { along: 397 });
    }

    #[test]
    fn entry_outside_the_display_is_clamped() {
        let c = Capture::enter((800, 600), (-50, 9000), Edge::Left);
        assert_eq!(c.position(), (0, 599));
    }

    #[test]
    fn all_borders_exit_for_windowed_inside() {
        let mut c = Capture::enter_with_rule((800, 600), (400, 300), ExitRule::AllBorders);
        assert_eq!(c.motion(-500, 0), Motion::Exit { along: 300 });
        assert_eq!(c.last_exit_edge(), Some(Edge::Left));

        let mut c = Capture::enter_with_rule((800, 600), (400, 300), ExitRule::AllBorders);
        assert_eq!(c.motion(500, 0), Motion::Exit { along: 300 });
        assert_eq!(c.last_exit_edge(), Some(Edge::Right));

        let mut c = Capture::enter_with_rule((800, 600), (400, 300), ExitRule::AllBorders);
        assert_eq!(c.motion(0, -500), Motion::Exit { along: 400 });
        assert_eq!(c.last_exit_edge(), Some(Edge::Top));

        let mut c = Capture::enter_with_rule((800, 600), (400, 300), ExitRule::AllBorders);
        assert_eq!(c.motion(0, 500), Motion::Exit { along: 400 });
        assert_eq!(c.last_exit_edge(), Some(Edge::Bottom));
    }

    #[test]
    fn linked_edges_for_fullscreen_inside() {
        // Only right edge is linked (e.g. to a right-side monitor)
        let rule = ExitRule::LinkedEdges {
            left: false,
            right: true,
            top: false,
            bottom: false,
        };
        let mut c = Capture::enter_with_rule((800, 600), (400, 300), rule);
        // Left is wall
        assert_eq!(c.motion(-500, 0), Motion::Inside { x: 0, y: 300 });
        // Right exits
        assert_eq!(c.motion(1000, 0), Motion::Exit { along: 300 });
        assert_eq!(c.last_exit_edge(), Some(Edge::Right));
    }
}
