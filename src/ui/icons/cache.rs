//! Retain icon geometry until its appearance or dimensions change.

use iced::{
    Renderer, Size,
    widget::canvas::{self, Frame, Geometry},
};
use std::cell::Cell;

pub(crate) struct Cache<Key> {
    key: Cell<Option<Key>>,
    geometry: canvas::Cache,
}

impl<Key> Default for Cache<Key> {
    fn default() -> Self {
        Self {
            key: Cell::new(None),
            geometry: canvas::Cache::default(),
        }
    }
}

impl<Key: Copy + PartialEq> Cache<Key> {
    pub(crate) fn draw(
        &self,
        key: Key,
        renderer: &Renderer,
        size: Size,
        draw: impl FnOnce(&mut Frame),
    ) -> Vec<Geometry> {
        if self.key.get() != Some(key) {
            self.geometry.clear();
            self.key.set(Some(key));
        }
        vec![self.geometry.draw(renderer, size, draw)]
    }
}
