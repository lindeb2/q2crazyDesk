//! The cursor while this computer's input is captured and sent to a target
//! (R-INPUT-2, R-LAYOUT-5, R-LAYOUT-6).
//!
//! While captured, the local cursor is held still and only motion deltas
//! arrive. This keeps a virtual cursor on the target's display in target
//! pixels. Pushing it through the one edge that links back to the controller
//! ends the capture; every other edge is a wall.

use q2_layout::Edge;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Still on the target display, at this pixel.
    Inside { x: i32, y: i32 },
    /// Left through the exit edge. `along` is the position along that edge
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
    exit: Edge,
}

impl Capture {
    /// Start a capture on a display of `size_px`, with the cursor at `entry_px`
    /// and `exit` as the edge that leads back.
    pub fn enter(size_px: (i32, i32), entry_px: (i32, i32), exit: Edge) -> Self {
        let width = size_px.0.max(1);
        let height = size_px.1.max(1);
        Self {
            width,
            height,
            x: entry_px.0.clamp(0, width - 1),
            y: entry_px.1.clamp(0, height - 1),
            exit,
        }
    }

    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn exit_edge(&self) -> Edge {
        self.exit
    }

    /// Apply a motion delta from the local mouse.
    pub fn motion(&mut self, dx: i32, dy: i32) -> Motion {
        let nx = self.x.saturating_add(dx);
        let ny = self.y.saturating_add(dy);
        let max_x = self.width - 1;
        let max_y = self.height - 1;
        let crossed = match self.exit {
            Edge::Left => nx < 0,
            Edge::Right => nx > max_x,
            Edge::Top => ny < 0,
            Edge::Bottom => ny > max_y,
        };
        self.x = nx.clamp(0, max_x);
        self.y = ny.clamp(0, max_y);
        if crossed {
            let along = if matches!(self.exit, Edge::Left | Edge::Right) {
                self.y
            } else {
                self.x
            };
            return Motion::Exit { along };
        }
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
}
