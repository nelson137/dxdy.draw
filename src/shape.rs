use std::sync::RwLock;

use super::pos::{Pos, PosOffset};

#[derive(Clone)]
pub(crate) struct Shape {
    start: Pos,
    verticies: Vec<PosOffset>,
}

impl Shape {
    pub(crate) const fn new() -> Self {
        Self {
            start: Pos::ZERO,
            verticies: Vec::new(),
        }
    }

    pub(crate) fn from_pos(x: f64, y: f64) -> Self {
        Self {
            start: Pos::new(x, y),
            verticies: Vec::new(),
        }
    }

    pub(crate) fn start(&self) -> Pos {
        self.start
    }

    pub(crate) fn verticies(&self) -> impl Iterator<Item = PosOffset> {
        self.verticies.iter().copied()
    }

    pub(crate) fn next_vertex(&mut self, x: f64, y: f64) {
        self.verticies.push(PosOffset::new(x, y));
    }
}

pub(crate) static ALL_SHAPES: RwLock<Vec<Shape>> = RwLock::new(Vec::new());
