#![no_std]

//! Version-one, platform-neutral presentation types.
//!
//! These types deliberately carry IDs and metadata only. Capability transfer,
//! page mapping, queue submission, and event delivery belong to the platform
//! adapters, never to this crate.

/// A server-issued surface identifier, scoped to a display session.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SurfaceId(pub u64);

/// A session-owned buffer identifier. It is not a memory address.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BufferId(pub u64);

/// The sole pixel format accepted by Cosmic Present v1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PixelFormat {
    Argb8888 = 1,
}

/// A non-negative rectangle in a buffer or output coordinate space.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub const fn right(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub const fn intersection(self, other: Self) -> Self {
        let left = if self.x > other.x { self.x } else { other.x };
        let top = if self.y > other.y { self.y } else { other.y };
        let right = if self.right() < other.right() {
            self.right()
        } else {
            other.right()
        };
        let bottom = if self.bottom() < other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };

        if right <= left || bottom <= top {
            Self::new(left, top, 0, 0)
        } else {
            Self::new(left, top, right - left, bottom - top)
        }
    }
}

/// Metadata for a client-owned, shared pixel buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct BufferDescriptor {
    pub id: BufferId,
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl BufferDescriptor {
    pub const BYTES_PER_PIXEL: u32 = 4;

    pub const fn bounds(self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }

    pub const fn is_valid(self) -> bool {
        self.width > 0
            && self.height > 0
            && self.stride >= self.width.saturating_mul(Self::BYTES_PER_PIXEL)
            && self.stride % Self::BYTES_PER_PIXEL == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_rectangles_at_the_edge() {
        assert_eq!(
            Rect::new(8, 8, 8, 8).intersection(Rect::new(0, 0, 12, 12)),
            Rect::new(8, 8, 4, 4),
        );
    }

    #[test]
    fn rejects_under_sized_argb_stride() {
        assert!(
            !BufferDescriptor {
                id: BufferId(1),
                format: PixelFormat::Argb8888,
                width: 20,
                height: 10,
                stride: 79,
            }
            .is_valid()
        );
    }
}
