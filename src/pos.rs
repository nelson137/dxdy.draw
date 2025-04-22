use std::ops;

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
    pub(crate) const ZERO: Self = Self { dx: 0., dy: 0. };

    pub(crate) const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }

    pub(crate) fn dist2(self) -> f64 {
        self.dx * self.dx + self.dy * self.dy
    }
}

impl ops::Add<PosOffset> for PosOffset {
    type Output = Self;

    fn add(self, rhs: PosOffset) -> Self::Output {
        Self::new(self.dx + rhs.dx, self.dy + rhs.dy)
    }
}

impl ops::Sub<PosOffset> for PosOffset {
    type Output = Self;

    fn sub(self, rhs: PosOffset) -> Self::Output {
        Self::new(self.dx - rhs.dx, self.dy - rhs.dy)
    }
}
