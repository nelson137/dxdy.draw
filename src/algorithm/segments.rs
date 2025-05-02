use std::{collections::HashMap, ops};

use super::zone_map::ZoneMap;

/// linked vertex segments optimized for differential growth-like operations
/// like spltting edges by inserting new vertices, and collapsing edges.
///
/// all vertices must exist within the unit square.
pub(super) struct Segments {
    /// TODO
    n_max: u64,
    /// TODO
    zone_width: f64,

    /// Number of vertices.
    v_num: u64,
    /// TODO
    v_act: u64,
    /// Number of edges.
    e_num: u64,
    /// Number of line segments.
    s_num: u64,

    /// TODO
    nz: u64,

    /// Map of vertex `x` coordinates by vertex index.
    pub(super) x: Vec<f64>,
    /// Map of vertex `y` coordinates by vertex index.
    pub(super) y: Vec<f64>,
    /// Map of vertex active status (0 or 1) by vertex index
    /// TODO: make this a `Vec<bool>`
    pub(super) va: Vec<i64>,
    /// Map of vertex to line segment index by vertex index.
    pub(super) vs: Vec<i64>,
    /// Map of edge to vertices (`v1` and `v2`) by edge index.
    /// `v1, v2 = self.ev[2 * e], self.ev[2 * e + 1]`
    pub(super) ev: Vec<i64>,
    /// TODO
    pub(super) ve: Vec<i64>,

    /// TODO
    pub(super) zone_map: ZoneMap,
}

//===================================================================
// Constructors
//===================================================================

impl Segments {
    /// initialize triangular mesh.
    ///
    /// - nmax is the maximal number of vertices/edges. storage is reserved upon
    ///   instantiation
    pub(super) fn new(n_max: u64, mut zone_width: f64) -> Self {
        let mut nz = zone_width.recip() as u64;
        if nz < 3 {
            nz = 1;
            zone_width = 1.0;
        }

        Self {
            n_max,
            zone_width,
            v_num: 0,
            v_act: 0,
            e_num: 0,
            s_num: 0,
            nz,
            x: vec![0.; n_max as usize],
            y: vec![0.; n_max as usize],
            va: vec![-1; n_max as usize],
            vs: vec![-1; n_max as usize],
            ev: vec![-1; 2 * n_max as usize],
            ve: vec![-1; 2 * n_max as usize],
            zone_map: ZoneMap::new(nz),
        }
    }
}

//===================================================================
// Helpers
//===================================================================

fn valid_new_vertex(x: f64, y: f64) -> bool {
    const R: ops::RangeInclusive<f64> = 0.0..=1.0;
    R.contains(&x) && R.contains(&y)
}

//===================================================================
// Private Methods
//===================================================================

impl Segments {
    fn add_vertex(&mut self, x: f64, y: f64, s: i64) -> i64 {
        if !valid_new_vertex(x, y) {
            panic!("Vertex is outside the unit square");
        }

        let v_num = self.v_num;

        self.x[v_num as usize] = x;
        self.y[v_num as usize] = y;
        self.va[v_num as usize] = 1;
        self.vs[v_num as usize] = s;

        self.zone_map.add_vertex(v_num, &self.x, &self.y);

        self.v_num += 1;
        v_num as i64
    }

    fn add_passive_vertex(&mut self, x: f64, y: f64, s: i64) -> i64 {
        if !valid_new_vertex(x, y) {
            panic!("Vertex is outside the unit square");
        }

        let v_num = self.v_num;

        self.x[v_num as usize] = x;
        self.y[v_num as usize] = y;
        self.va[v_num as usize] = 0;
        self.vs[v_num as usize] = s;

        self.zone_map.add_vertex(v_num, &self.x, &self.y);

        self.v_num += 1;
        v_num as i64
    }

    fn valid_new_edge(&self, v1: i64, v2: i64) -> bool {
        let r = 0..self.v_num as i64;
        r.contains(&v1)
            && r.contains(&v2)
            && self.va[v1 as usize] >= 0
            && self.va[v2 as usize] >= 0
    }

    /// add edge between vertices v1 and v2. returns id of new edge
    fn add_edge(&mut self, v1: i64, v2: i64) -> i64 {
        if !self.valid_new_edge(v1, v2) {
            panic!("invalid vertex: v{v1} -> v{v2}");
        }

        let e_num = self.e_num;

        self.ev[2 * e_num as usize] = v1;
        self.ev[2 * e_num as usize + 1] = v2;

        self.add_e_to_ve(v1, e_num as i64);
        self.add_e_to_ve(v2, e_num as i64);

        self.e_num += 1;
        e_num as i64
    }

    #[inline(always)]
    fn add_e_to_ve(&mut self, v: i64, e: i64) {
        let v = v as usize;
        if self.ve[2 * v] < 0 {
            self.ve[2 * v] = e;
        } else {
            self.ve[2 * v + 1] = e;
        }
    }

    fn edge_exists(&self, e1: i64) -> bool {
        let e1 = e1 as usize;
        self.ev[2 * e1] > -1 && self.ev[2 * e1 + 1] > -1
    }

    fn vertex_exists(&self, v1: i64) -> bool {
        self.va[v1 as usize] > -1
    }

    fn vertex_status(&self, v1: i64) -> i64 {
        self.va[v1 as usize]
    }

    fn vertex_segment(&self, v1: i64) -> i64 {
        self.vs[v1 as usize]
    }

    fn delete_vertex(&mut self, v1: i64) {
        self.va[v1 as usize] = -1;
        self.zone_map.delete_vertex(v1);
    }

    fn set_passive_vertex(&mut self, v1: i64) {
        self.va[v1 as usize] = 0;
    }

    fn delete_edge(&mut self, e1: i64) {
        if e1 < 0 || e1 >= self.e_num as i64 {
            panic!("invalid edge: e{e1}");
        }

        let i = 2 * e1 as usize;
        let (v1, v2) = (self.ev[i], self.ev[i + 1]);
        self.ev[i] = -1;
        self.ev[i + 1] = -1;

        if v1 > -1 {
            self.delete_e_from_ve(v1, e1);
        }
        if v2 > -1 {
            self.delete_e_from_ve(v2, e1);
        }
    }

    #[inline(always)]
    fn delete_e_from_ve(&mut self, v: i64, e: i64) {
        let v = v as usize;
        if self.ve[2 * v] == e {
            self.ve[2 * v] = self.ve[2 * v + 1];
            self.ve[2 * v + 1] = -1;
        } else if self.ve[2 * v + 1] == e {
            self.ve[2 * v + 1] = -1;
        }
    }

    // fn get_edge_normal(&self, s1: i64, normals: &mut [f64]) {}
}

//===================================================================
// Public Methods
//===================================================================

impl Segments {
    // pub(super) fn get_edges_coordinates(&self) -> Vec<[f64; 4]>

    /// get all coordinates x1,y1,x2,y2 of all edges
    /// buf = [[x1,y1,x2,y2], ...]
    pub(super) fn np_get_edges_coordinates(
        &self,
        buf: &mut [[f64; 4]],
    ) -> usize {
        let mut n = 0;

        for e in 0..self.e_num as usize {
            if self.ev[2 * e] > -1 {
                let (v1, v2) = (self.ev[2 * e], self.ev[2 * e + 1]);
                buf[n] = [
                    self.x[v1 as usize],
                    self.y[v1 as usize],
                    self.x[v2 as usize],
                    self.y[v2 as usize],
                ];

                n += 1;
            }
        }

        n
    }

    pub(super) fn np_get_edges(&self, buf: &mut [[i64; 2]]) -> usize {
        let mut n = 0;

        for e in 0..self.e_num as usize {
            if self.ev[2 * e] > -1 {
                buf[n] = [self.ev[2 * e], self.ev[2 * e + 1]];
                n += 1;
            }
        }

        n
    }

    /// get all coordinates x1,y1 of all alive vertices
    /// buf = [[x1,y1], ...]
    pub(super) fn np_get_vertex_coordinates(
        &self,
        buf: &mut [[f64; 2]],
    ) -> usize {
        let mut n = 0;

        for v in 0..self.v_num as usize {
            if self.va[v] > -1 {
                buf[n] = [self.x[v], self.y[v]];
                n += 1;
            }
        }

        n
    }

    pub(super) fn get_greatest_distance(&self, x: f64, y: f64) -> f64 {
        let mut max_dist: f64 = 0.0;

        for v in 0..self.v_num as usize {
            if self.va[v] > -1 {
                let (dx, dy) = (x - self.x[v], y - self.y[v]);
                // TODO: wait to sqrt until after the loop
                let dist = dx.hypot(dy);
                if dist > max_dist {
                    max_dist = dist;
                }
            }
        }

        max_dist
    }

    pub(super) fn np_get_sorted_vertices(&self, buf: &mut [i64]) -> usize {
        let mut e_start = usize::MAX;

        let mut ev_array = vec![[-1_i64, -1_i64]; self.e_num as usize];
        let mut ve_map = HashMap::<i64, Vec<usize>>::new();

        let mut e_visited = vec![false; self.e_num as usize];
        let mut v_ordered = Vec::<i64>::new();

        for e in 0..self.e_num as usize {
            if self.ev[2 * e] > -1 {
                e_start = e;

                let (v1, v2) = (self.ev[2 * e], self.ev[2 * e + 1]);
                ev_array[e] = [v1, v2];

                ve_map.entry(v1).or_default().push(e);
                ve_map.entry(v2).or_default().push(e);
            }
        }

        if e_start < usize::MAX {
            e_visited[e_start] = true;

            let [v_end, mut v_cur] = ev_array[e_start];

            while v_cur != v_end {
                let ve = &**ve_map.get(&v_cur).unwrap();
                let e = if e_visited[ve[0]] { ve[1] } else { ve[0] };
                e_visited[e] = true;

                let [v1, v2] = ev_array[e];
                v_cur = if v1 == v_cur { v2 } else { v1 };

                v_ordered.push(v_cur);
            }
        }

        buf.copy_from_slice(&v_ordered);

        v_ordered.len()
    }

    /// TODO: these docs may not be accurate
    ///
    /// get list of lists with coordinates x1,y1,x2,y2 of all edges
    ///
    /// list is sorted, and this only works if we have one single closed
    /// segment.
    pub(super) fn np_get_sorted_vertex_coordinates(
        &self,
        buf: &mut [[f64; 2]],
    ) -> usize {
        let mut e_start = usize::MAX;

        let mut ev_array = vec![[-1_i64, -1_i64]; self.e_num as usize];
        let mut ve_map = HashMap::<i64, Vec<usize>>::new();

        let mut e_visited = vec![false; self.e_num as usize];
        let mut v_ordered = Vec::<i64>::new();

        for e in 0..self.e_num as usize {
            if self.ev[2 * e] > -1 {
                e_start = e;

                let (v1, v2) = (self.ev[2 * e], self.ev[2 * e + 1]);
                ev_array[e] = [v1, v2];

                ve_map.entry(v1).or_default().push(e);
                ve_map.entry(v2).or_default().push(e);
            }
        }

        if e_start < usize::MAX {
            e_visited[e_start] = true;

            let [v_end, mut v_cur] = ev_array[e_start];

            while v_cur != v_end {
                let ve = &**ve_map.get(&v_cur).unwrap();
                let e = if e_visited[ve[0]] { ve[1] } else { ve[0] };
                e_visited[e] = true;

                let [v1, v2] = ev_array[e];
                v_cur = if v1 == v_cur { v2 } else { v1 };

                v_ordered.push(v_cur);
            }
        }

        for (i, v) in v_ordered.iter().copied().enumerate() {
            buf[i] = [self.x[v as usize], self.y[v as usize]];
        }

        v_ordered.len()
    }

    pub(super) fn get_edges(&self) -> Vec<i64> {
        (0..self.e_num as usize)
            .filter(|&e| self.ev[2 * e] > -1)
            .map(|e| e as i64)
            .collect()
    }

    pub(super) fn get_edges_vertices(&self) -> Vec<[i64; 2]> {
        (0..self.e_num as usize)
            .filter(|&e| self.ev[2 * e] > -1)
            .map(|e| [self.ev[2 * e], self.ev[2 * e + 1]])
            .collect()
    }

    pub(super) fn get_edge_length(&self, e1: i64) -> f64 {
        let e1 = e1 as usize;
        let nx = self.x[self.ev[2 * e1] as usize]
            - self.x[self.ev[2 * e1 + 1] as usize];
        let ny = self.y[self.ev[2 * e1] as usize]
            - self.y[self.ev[2 * e1 + 1] as usize];
        nx.hypot(ny)
    }

    pub(super) fn get_edge_vertices(&self, e1: i64) -> [i64; 2] {
        let e1 = e1 as usize;
        [self.ev[2 * e1], self.ev[2 * e1 + 1]]
    }

    pub(super) fn init_line_segment(
        &mut self,
        xys: &[[f64; 2]],
        lock_edges: bool,
    ) {
        let s_num = self.s_num as i64;
        // TODO(optimize): this vec is not needed
        let mut vertices = Vec::<i64>::new();

        if lock_edges {
            vertices.push({
                let [x, y] = xys[0];
                self.add_passive_vertex(x, y, s_num)
            });
            for &[x, y] in &xys[1..xys.len() - 1] {
                vertices.push(self.add_vertex(x, y, s_num));
            }
            vertices.push({
                let [x, y] = xys[xys.len() - 1];
                self.add_passive_vertex(x, y, s_num)
            });
        } else {
            for &[x, y] in xys {
                vertices.push(self.add_vertex(x, y, s_num));
            }
        }

        for e in vertices.chunks_exact(2) {
            self.add_edge(e[0], e[1]);
        }

        self.s_num += 1;
    }

    pub(super) fn init_passive_line_segment(&mut self, xys: &[[f64; 2]]) {
        let s_num = self.s_num as i64;
        // TODO(optimize): this vec is not needed
        let mut vertices = Vec::<i64>::new();

        for &[x, y] in xys {
            vertices.push(self.add_passive_vertex(x, y, s_num));
        }

        for e in vertices.chunks_exact(2) {
            self.add_edge(e[0], e[1]);
        }

        self.s_num += 1;
    }

    pub(super) fn init_circle_segment(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        angles: &[f64],
    ) {
        let s_num = self.s_num as i64;
        // TODO(optimize): this vec is not needed
        let mut vertices = Vec::<i64>::new();

        for &theta in angles {
            vertices.push(self.add_vertex(
                x + r * theta.cos(),
                y + r * theta.sin(),
                s_num,
            ));
        }

        for e in vertices.chunks_exact(2) {
            self.add_edge(e[0], e[1]);
        }

        self.add_edge(vertices[0], vertices[vertices.len() - 1]);

        self.s_num += 1;
    }

    pub(super) fn init_passive_circle_segment(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        angles: &[f64],
    ) {
        let s_num = self.s_num as i64;
        // TODO(optimize): this vec is not needed
        let mut vertices = Vec::<i64>::new();

        for &theta in angles {
            vertices.push(self.add_passive_vertex(
                x + r * theta.cos(),
                y + r * theta.sin(),
                s_num,
            ));
        }

        for e in vertices.chunks_exact(2) {
            self.add_edge(e[0], e[1]);
        }

        self.add_edge(vertices[0], vertices[vertices.len() - 1]);

        self.s_num += 1;
    }

    /// ## Panics
    ///
    /// Panics if `max_len > 0.` and the edge length is greater than `max_len`.
    ///
    /// Use [`Self::collapse_edge_no_max`] to collapse with no upper bound.
    pub(super) fn collapse_edge(&mut self, e1: i64, max_len: f64) {
        if e1 < 0 {
            panic!("invalid edge: e{e1}");
        }
        if !self.edge_exists(e1) {
            panic!("edge does not exist: e{e1}");
        }

        let (v1, v2) =
            (self.ev[2 * e1 as usize], self.ev[2 * e1 as usize + 1]);

        if self.va[v1 as usize] < 1 {
            panic!("edge has passive vertex: e{e1} | *v{v1}* -> v{v2}");
        }
        if self.va[v2 as usize] < 1 {
            panic!("edge has passive vertex: e{e1} | v{v1} -> *v{v2}*");
        }

        let e2 = if self.ve[2 * v1 as usize] == e1 {
            self.ve[2 * v1 as usize + 1]
        } else {
            self.ve[2 * v1 as usize]
        };

        let v3 = if self.ev[2 * e2 as usize] == v1 {
            self.ev[2 * e2 as usize + 1]
        } else {
            self.ev[2 * e2 as usize]
        };

        if max_len > 0. {
            let dx = self.x[v1 as usize] - self.x[v2 as usize];
            let dy = self.y[v1 as usize] - self.y[v2 as usize];
            let dist2 = dx * dx + dy * dy;
            if dist2 > max_len * max_len {
                panic!(
                    "cannot collapse edge longer than the maximum: e{e1}, len={:.4}, max={max_len:.4}",
                    dist2.sqrt()
                );
            }
        }

        self.x[v2 as usize] = (self.x[v1 as usize] + self.x[v2 as usize]) / 2.;
        self.y[v2 as usize] = (self.y[v1 as usize] + self.y[v2 as usize]) / 2.;

        self.delete_edge(e1);
        self.delete_edge(e2);

        self.delete_vertex(v1);
        self.add_edge(v3, v2);
    }

    pub(super) fn collapse_edge_no_max(&mut self, e1: i64) {
        self.collapse_edge(e1, -1.)
    }

    /// ## Panics
    ///
    /// Panics if `min_len > 0.` and the edge length is less than `min_len`.
    ///
    /// Use [`Self::split_edge_no_min`] to split with no lower bound.
    pub(super) fn split_edge(
        &mut self,
        e1: i64,
        min_len: f64,
    ) -> Result<(), ()> {
        if e1 < 0 {
            eprintln!("invalid edge: e{e1}");
            return Err(());
        }
        if !self.edge_exists(e1) {
            eprintln!("edge does not exist: e{e1}");
            return Err(());
        }

        let (v1, v2) =
            (self.ev[2 * e1 as usize], self.ev[2 * e1 as usize + 1]);

        let s = self.vs[v1 as usize];
        if s < 0 {
            eprintln!("invalid segment: e{e1} | v{v1}");
            return Err(());
        }

        if min_len > 0. {
            let dx = self.x[v1 as usize] - self.x[v2 as usize];
            let dy = self.y[v1 as usize] - self.y[v2 as usize];
            let dist2 = dx * dx + dy * dy;
            if dist2 < min_len * min_len {
                panic!(
                    "cannot split edge shorter than the minimum: e{e1}, len={:.4}, min={min_len:.4}",
                    dist2.sqrt()
                );
            }
        }

        let mid_x = (self.x[v1 as usize] + self.x[v2 as usize]) / 2.;
        let mid_y = (self.y[v1 as usize] + self.y[v2 as usize]) / 2.;

        let v3 = self.add_vertex(mid_x, mid_y, s);
        self.delete_edge(e1);

        self.add_edge(v1, v3);
        self.add_edge(v2, v3);

        Ok(())
    }

    pub(super) fn split_edge_no_min(&mut self, e1: i64) -> Result<(), ()> {
        self.split_edge(e1, -1.)
    }

    /// split all edges longer than limit
    pub(super) fn split_long_edges(&mut self, limit: f64) {
        for e in 0..self.e_num as i64 {
            if self.ev[2 * e as usize] > -1 {
                let (v1, v2) =
                    (self.ev[2 * e as usize], self.ev[2 * e as usize + 1]);
                if self.va[v1 as usize] < 1 && self.va[v2 as usize] < 1 {
                    continue; // edge is passive/dead
                }

                let dx = self.x[v1 as usize] - self.x[v2 as usize];
                let dy = self.y[v1 as usize] - self.y[v2 as usize];
                let dist = dx.hypot(dy);
                if dist > limit {
                    // TODO: handle error ??
                    _ = self.split_edge_no_min(e);
                }
            }
        }
    }

    /// Gives an estimate of edge, e1, using the cross product of e1 and both the
    /// connected edges of e1. This is not really the curvature in the mathematical
    /// sense.
    pub(super) fn get_edge_curvature(&self, e1: i64) -> f64 {
        if e1 < 0 {
            panic!("invalid edge: e{e1}");
        }
        if !self.edge_exists(e1) {
            panic!("edge does not exist: e{e1}");
        }
        let e1 = e1 as usize;

        let (v1, v2) = (self.ev[2 * e1], self.ev[2 * e1 + 1]);
        if v1 < 0 {
            panic!("invalid vertex for edge: e{e1} | v1={v1}");
        }
        if v2 < 0 {
            panic!("invalid vertex for edge: e{e1} | v2={v2}");
        }
        let (v1, v2) = (v1 as usize, v2 as usize);

        let (e2, e3) = if self.ve[2 * v1] == self.ve[2 * v2] {
            (self.ve[2 * v1 + 1], self.ve[2 * v2 + 1])
        } else if self.ve[2 * v1] == self.ve[2 * v2 + 1] {
            (self.ve[2 * v1 + 1], self.ve[2 * v2])
        } else if self.ve[2 * v1 + 1] == self.ve[2 * v2] {
            (self.ve[2 * v1], self.ve[2 * v2 + 1])
        } else if self.ve[2 * v1 + 1] == self.ve[2 * v2 + 1] {
            (self.ve[2 * v1], self.ve[2 * v2])
        } else {
            panic!("edges are not connected")
        };

        let (v3, v4) = (self.ev[2 * e1], self.ev[2 * e1 + 1]);
        let mut t: f64 = 0.0;

        if e2 > -1 {
            let (v1, v2) =
                (self.ev[2 * e2 as usize], self.ev[2 * e2 as usize + 1]);
            let ax = self.x[v1 as usize] - self.x[v2 as usize];
            let bx = self.x[v3 as usize] - self.x[v4 as usize];
            let ay = self.y[v1 as usize] - self.y[v2 as usize];
            let by = self.y[v3 as usize] - self.y[v4 as usize];
            t += (ax * by - ay * bx).abs() / 2.;
        }

        if e3 > -1 {
            let (v1, v2) =
                (self.ev[2 * e3 as usize], self.ev[2 * e3 as usize + 1]);
            let ax = self.x[v1 as usize] - self.x[v2 as usize];
            let bx = self.x[v3 as usize] - self.x[v4 as usize];
            let ay = self.y[v1 as usize] - self.y[v2 as usize];
            let by = self.y[v3 as usize] - self.y[v4 as usize];
            t += (ax * by - ay * bx).abs() / 2.;
        }

        if t <= 0. {
            panic!("no curvature");
        }

        t
    }

    pub(super) fn get_active_vertex_count(&self) -> usize {
        self.va
            .iter()
            .copied()
            .take(self.v_num as usize)
            .filter(|a| *a > 0)
            .count()
    }

    /// check that all vertices are within limit of unit square boundary
    pub(super) fn safe_vertex_positions(&self, limit: f64) -> bool {
        let range = limit..=1. - limit;

        for i in 0..self.v_num as usize {
            let (x, y) = (self.x[i], self.y[i]);
            if !range.contains(&x) || !range.contains(&y) {
                return false;
            }
        }

        true
    }

    // pub(super) fn s_num(&self) -> u64 {
    //     self.s_num
    // }

    pub(super) fn v_num(&self) -> u64 {
        self.v_num
    }

    pub(super) fn e_num(&self) -> u64 {
        self.e_num
    }
}
