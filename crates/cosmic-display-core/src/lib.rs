#![no_std]

//! The policy-free state needed by a simple compositor.
//!
//! It intentionally has no event loop, syscalls, filesystem access, renderer,
//! or device access. A server adapter supplies those concerns.

extern crate alloc;

use alloc::vec::Vec;
use cosmic_present::{BufferDescriptor, BufferId, Rect, SurfaceId};

/// An ordering band retained from Orbital's window stack.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SurfaceLayer {
    Back,
    Normal,
    Front,
}

/// A rendering plan entry. The order is front to back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceView {
    pub id: SurfaceId,
    pub layer: SurfaceLayer,
    pub rect: Rect,
    pub buffer: Option<BufferId>,
    pub focused: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Surface {
    id: SurfaceId,
    layer: SurfaceLayer,
    rect: Rect,
    visible: bool,
    buffer: Option<BufferDescriptor>,
    damage: Vec<Rect>,
}

/// Failures that must be reported by an adapter as a protocol error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceError {
    DuplicateSurface,
    UnknownSurface,
    InvalidBuffer,
}

/// Surface order, keyboard focus, attachments, and pending damage.
///
/// The focus list is distinct from the layer: focusing a background surface
/// makes it receive input but does not change its rendering band.
#[derive(Default)]
pub struct SurfaceStack {
    surfaces: Vec<Surface>,
    focus_order: Vec<SurfaceId>,
}

impl SurfaceStack {
    pub const fn new() -> Self {
        Self {
            surfaces: Vec::new(),
            focus_order: Vec::new(),
        }
    }

    pub fn create(
        &mut self,
        id: SurfaceId,
        rect: Rect,
        layer: SurfaceLayer,
    ) -> Result<(), SurfaceError> {
        if self.find(id).is_some() {
            return Err(SurfaceError::DuplicateSurface);
        }

        self.surfaces.push(Surface {
            id,
            layer,
            rect,
            visible: true,
            buffer: None,
            damage: Vec::new(),
        });

        match layer {
            SurfaceLayer::Back => self.focus_order.push(id),
            SurfaceLayer::Normal | SurfaceLayer::Front => self.focus_order.insert(0, id),
        }
        Ok(())
    }

    pub fn destroy(&mut self, id: SurfaceId) -> Result<(), SurfaceError> {
        let index = self.find_index(id).ok_or(SurfaceError::UnknownSurface)?;
        self.surfaces.remove(index);
        self.focus_order.retain(|candidate| *candidate != id);
        Ok(())
    }

    pub fn set_visible(&mut self, id: SurfaceId, visible: bool) -> Result<(), SurfaceError> {
        self.find_mut(id)
            .ok_or(SurfaceError::UnknownSurface)?
            .visible = visible;
        Ok(())
    }

    pub fn focus(&mut self, id: SurfaceId) -> Result<(), SurfaceError> {
        let surface = self.find(id).ok_or(SurfaceError::UnknownSurface)?;
        if !surface.visible {
            return Ok(());
        }

        self.focus_order.retain(|candidate| *candidate != id);
        self.focus_order.insert(0, id);
        Ok(())
    }

    pub fn focused(&self) -> Option<SurfaceId> {
        self.focus_order
            .iter()
            .copied()
            .find(|id| self.find(*id).is_some_and(|surface| surface.visible))
    }

    /// Attach a validated client buffer and retain only damage inside it.
    pub fn commit(
        &mut self,
        id: SurfaceId,
        buffer: BufferDescriptor,
        damage: &[Rect],
    ) -> Result<(), SurfaceError> {
        if !buffer.is_valid() {
            return Err(SurfaceError::InvalidBuffer);
        }

        let surface = self.find_mut(id).ok_or(SurfaceError::UnknownSurface)?;
        let bounds = buffer.bounds();
        surface.buffer = Some(buffer);
        surface.damage.clear();
        surface.damage.extend(
            damage
                .iter()
                .copied()
                .map(|rect| rect.intersection(bounds))
                .filter(|rect| !rect.is_empty()),
        );
        Ok(())
    }

    pub fn take_damage(&mut self, id: SurfaceId) -> Result<Vec<Rect>, SurfaceError> {
        Ok(core::mem::take(
            &mut self
                .find_mut(id)
                .ok_or(SurfaceError::UnknownSurface)?
                .damage,
        ))
    }

    /// Return visible surfaces in deterministic front-to-back paint order.
    pub fn front_to_back(&self) -> Vec<SurfaceView> {
        let focused = self.focused();
        let mut ordered: Vec<_> = self
            .focus_order
            .iter()
            .filter_map(|id| self.find(*id))
            .filter(|surface| surface.visible)
            .map(|surface| SurfaceView {
                id: surface.id,
                layer: surface.layer,
                rect: surface.rect,
                buffer: surface.buffer.map(|buffer| buffer.id),
                focused: Some(surface.id) == focused,
            })
            .collect();
        ordered.sort_by(|left, right| right.layer.cmp(&left.layer));
        ordered
    }

    fn find(&self, id: SurfaceId) -> Option<&Surface> {
        self.surfaces.iter().find(|surface| surface.id == id)
    }

    fn find_mut(&mut self, id: SurfaceId) -> Option<&mut Surface> {
        self.surfaces.iter_mut().find(|surface| surface.id == id)
    }

    fn find_index(&self, id: SurfaceId) -> Option<usize> {
        self.surfaces.iter().position(|surface| surface.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    const A: SurfaceId = SurfaceId(1);
    const B: SurfaceId = SurfaceId(2);
    const C: SurfaceId = SurfaceId(3);

    fn rect() -> Rect {
        Rect::new(0, 0, 100, 100)
    }

    fn buffer(id: u64) -> BufferDescriptor {
        BufferDescriptor {
            id: BufferId(id),
            format: cosmic_present::PixelFormat::Argb8888,
            width: 100,
            height: 100,
            stride: 400,
        }
    }

    #[test]
    fn keeps_layers_but_focuses_the_selected_normal_surface() {
        let mut stack = SurfaceStack::new();
        stack.create(A, rect(), SurfaceLayer::Normal).unwrap();
        stack.create(B, rect(), SurfaceLayer::Back).unwrap();
        stack.create(C, rect(), SurfaceLayer::Front).unwrap();
        stack.focus(A).unwrap();

        let plan = stack.front_to_back();
        assert_eq!(
            plan.iter().map(|view| view.id).collect::<Vec<_>>(),
            vec![C, A, B]
        );
        assert_eq!(stack.focused(), Some(A));
        assert!(plan.iter().find(|view| view.id == A).unwrap().focused);
    }

    #[test]
    fn clips_damage_to_the_committed_buffer() {
        let mut stack = SurfaceStack::new();
        stack.create(A, rect(), SurfaceLayer::Normal).unwrap();
        stack
            .commit(
                A,
                buffer(7),
                &[Rect::new(90, 90, 20, 20), Rect::new(150, 0, 1, 1)],
            )
            .unwrap();

        assert_eq!(
            stack.take_damage(A).unwrap(),
            vec![Rect::new(90, 90, 10, 10)]
        );
    }

    #[test]
    fn destroying_the_focused_surface_releases_focus_and_render_plan_entry() {
        let mut stack = SurfaceStack::new();
        stack.create(A, rect(), SurfaceLayer::Normal).unwrap();
        stack.create(B, rect(), SurfaceLayer::Normal).unwrap();
        stack.focus(A).unwrap();
        stack.destroy(A).unwrap();

        assert_eq!(stack.focused(), Some(B));
        assert_eq!(
            stack
                .front_to_back()
                .iter()
                .map(|view| view.id)
                .collect::<Vec<_>>(),
            vec![B]
        );
    }
}
