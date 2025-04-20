#[derive(Clone, Copy, Default)]
pub(crate) struct Pos {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl Pos {
    pub(crate) const ZERO: Self = Self { x: 0., y: 0. };

    pub(crate) const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct PosOffset {
    pub(crate) dx: f64,
    pub(crate) dy: f64,
}

impl PosOffset {
    pub(crate) const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }
}
