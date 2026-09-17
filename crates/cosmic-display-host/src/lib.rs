//! Host-only observability scaffolding.
//!
//! This is deliberately a recorder, not a graphical demo or a substitute for
//! a QDX-G card. It gives the portable core deterministic host evidence while
//! preserving the later native hardware acceptance boundary.

use cosmic_display_core::{SurfaceStack, SurfaceView};
use cosmic_present::Rect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordedFrame {
    pub front_to_back: Vec<SurfaceView>,
    pub damage: Vec<Rect>,
}

#[derive(Default)]
pub struct MockOutput {
    frames: Vec<RecordedFrame>,
}

impl MockOutput {
    pub fn record(&mut self, stack: &SurfaceStack, damage: Vec<Rect>) {
        self.frames.push(RecordedFrame {
            front_to_back: stack.front_to_back(),
            damage,
        });
    }

    pub fn frames(&self) -> &[RecordedFrame] {
        &self.frames
    }
}

#[cfg(test)]
mod tests {
    use cosmic_display_core::{SurfaceLayer, SurfaceStack};
    use cosmic_present::{Rect, SurfaceId};

    use super::MockOutput;

    #[test]
    fn records_exact_compositor_order_and_damage() {
        let mut stack = SurfaceStack::new();
        stack
            .create(SurfaceId(1), Rect::new(0, 0, 10, 10), SurfaceLayer::Normal)
            .unwrap();
        stack
            .create(SurfaceId(2), Rect::new(0, 0, 10, 10), SurfaceLayer::Front)
            .unwrap();

        let mut output = MockOutput::default();
        output.record(&stack, vec![Rect::new(1, 2, 3, 4)]);

        assert_eq!(output.frames()[0].front_to_back[0].id, SurfaceId(2));
        assert_eq!(output.frames()[0].damage, vec![Rect::new(1, 2, 3, 4)]);
    }
}
