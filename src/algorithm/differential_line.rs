pub(super) struct DifferentialLine {
    pub(super) segments: super::segments::Segments,

    /// the closest comfortable distance between two vertices.
    near_l: f64,
    /// the distance beyond which disconnected vertices will ignore each other
    far_l: f64,

    sx: Vec<f64>,
    sy: Vec<f64>,
    sd: Vec<u64>,
    vertices: Vec<u64>,
}

//===================================================================
// Constructors
//===================================================================

impl DifferentialLine {
    pub(super) fn new(
        n_max: u64,
        zone_width: f64,
        near_l: f64,
        far_l: f64,
    ) -> Self {
        Self {
            segments: super::segments::Segments::new(n_max, zone_width),
            near_l,
            far_l,
            sx: Vec::with_capacity(n_max as usize),
            sy: Vec::with_capacity(n_max as usize),
            sd: Vec::with_capacity(n_max as usize),
            vertices: Vec::with_capacity(n_max as usize),
        }
    }
}

//===================================================================
// Private Methods
//===================================================================

impl DifferentialLine {
    /// all vertices will move away from all neighboring (closer than farl)
    /// vertices
    ///
    /// TODO: are `vertices`, `sx`, and `sy` not from `self` ??
    fn reject(
        &mut self,
        v: i64,
        vertices: &[i64],
        n_vertices: usize,
        step: f64,
    ) -> bool {
        if self.segments.va[v as usize] < 1 {
            return false;
        }

        let (e1, e2) = (
            self.segments.ve[2 * v as usize],
            self.segments.ve[2 * v as usize + 1],
        );

        let v1 = if self.segments.ev[2 * e1 as usize] == v {
            self.segments.ev[2 * e1 as usize + 1]
        } else {
            self.segments.ev[2 * e1 as usize]
        };

        let v2 = if self.segments.ev[2 * e2 as usize] == v {
            self.segments.ev[2 * e2 as usize + 1]
        } else {
            self.segments.ev[2 * e2 as usize]
        };

        let (mut res_x, mut res_y): (f64, f64) = (0., 0.);

        for neighbor in vertices.iter().copied().take(n_vertices) {
            let dx = self.segments.x[v as usize]
                - self.segments.x[neighbor as usize];
            let dy = self.segments.y[v as usize]
                - self.segments.y[neighbor as usize];
            let norm = dx.hypot(dy);

            if neighbor == v1 || neighbor == v2 {
                // linked

                if norm < self.near_l || norm <= 0. {
                    continue;
                }

                res_x += step * -dx / norm;
                res_y += step * -dy / norm;
            } else {
                // not linked

                if norm > self.far_l || norm <= 0. {
                    continue;
                }

                res_x += step * dx * (self.far_l / norm - 1.);
                res_y += step * dy * (self.far_l / norm - 1.);
            }

            self.sx[v as usize] += res_x;
            self.sy[v as usize] += res_y;
        }

        true
    }
}

//===================================================================
// Public Methods
//===================================================================

impl DifferentialLine {
    pub(super) fn optimize_position(&mut self, step: f64) {
        let mut vertices = Vec::<i64>::with_capacity(
            self.segments.zone_map.get_max_sphere_count() as usize,
        );

        for v in 0..self.segments.v_num() as i64 {
            self.sx[v as usize] = 0.;
            self.sy[v as usize] = 0.;

            let n_vertices = self.segments.zone_map.sphere_vertices(
                v,
                &self.segments.x,
                &self.segments.y,
                self.far_l,
                &mut vertices,
            );

            self.reject(v, &vertices, n_vertices, step);
        }

        for v in 0..self.segments.v_num() as usize {
            if self.segments.va[v] < 0 {
                continue;
            }

            self.segments.x[v] += self.sx[v];
            self.segments.y[v] += self.sy[v];
        }

        for v in 0..self.segments.v_num() {
            if self.segments.va[v as usize] < 0 {
                continue;
            }

            self.segments.zone_map.update_vertex(
                v as i64,
                self.segments.x[v as usize],
                self.segments.y[v as usize],
            );
        }
    }
}
