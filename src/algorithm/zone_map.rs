const SIZE: u64 = 1024;

struct Sz {
    i: u64,
    size: u64,
    count: u64,
    zv: Vec<i64>,
}

impl Sz {
    fn add_vertex(&mut self, v1: i64) {
        self.zv[self.count as usize] = v1;
        self.count += 1;
    }
}

pub(super) struct ZoneMap {
    v_num: u64,
    v_size: u64,
    nz: u64,
    total_zones: u64,
    greatest_zone_size: u64,
    /// Map of vertex to `z` by vertex index.
    vz: Vec<i64>,
    z: Vec<Sz>,
}

//===================================================================
// Constructors
//===================================================================

impl ZoneMap {
    pub(super) fn new(nz: u64) -> Self {
        let total_zones = nz * nz;

        let mut z = Vec::with_capacity(total_zones as usize);
        for i in 0..total_zones {
            z.push(Sz {
                i,
                size: SIZE,
                count: 0,
                zv: Vec::with_capacity(SIZE as usize),
            });
        }

        Self {
            v_num: 0,
            v_size: SIZE,
            nz,
            total_zones,
            greatest_zone_size: SIZE,
            vz: Vec::with_capacity(SIZE as usize),
            z,
        }
    }
}

//===================================================================
// Private Methods
//===================================================================

impl ZoneMap {
    fn add_vertex_to_zone(&mut self, z1: i64, v1: i64) {
        let sz = &mut self.z[z1 as usize];

        sz.add_vertex(v1);

        if sz.count >= sz.size - 1 {
            // zonemap.pyx:151:__extend_zv_of_zone()
            // TODO: deleteme, simulating realloc
            sz.size *= 2;
            if sz.size > self.greatest_zone_size {
                self.greatest_zone_size = sz.size;
            }
        }
    }

    fn remove_vertex_from_zone(&mut self, z1: i64, v1: i64) {
        let sz = &mut self.z[z1 as usize];

        for i in 0..sz.count as usize {
            if sz.zv[i] == v1 {
                sz.zv[i] = sz.zv[sz.count as usize - 1];
                sz.count -= 1;
                return;
            }
        }
    }

    fn get_z(&self, x: f64, y: f64) -> i64 {
        let nz = self.nz as i64;
        let i = x as i64 * nz;
        let j = y as i64 * nz;
        nz * i + j
    }
}

//===================================================================
// Public Methods
//===================================================================

impl ZoneMap {
    pub(super) fn add_vertex(
        &mut self,
        v1: u64,
        xs: &[f64],
        ys: &[f64],
    ) -> u64 {
        let v_num = self.v_num;

        let (x, y) = (xs[v1 as usize], ys[v1 as usize]);

        let z1 = self.get_z(x, y);
        self.add_vertex_to_zone(z1, v_num as i64);
        self.vz[v_num as usize] = z1;

        // TODO: deleteme, simulating realloc
        if v_num >= self.v_size - 1 {
            self.v_size *= 2;
        }

        self.v_num += 1;
        v_num
    }

    pub(super) fn delete_vertex(&mut self, v1: i64) {
        self.remove_vertex_from_zone(self.vz[v1 as usize], v1);
        self.vz[v1 as usize] = -1;
    }

    pub(super) fn get_max_sphere_count(&self) -> u64 {
        self.greatest_zone_size * 9
    }

    pub(super) fn sphere_vertices(
        &self,
        v: i64,
        xs: &[f64],
        ys: &[f64],
        rad: f64,
        vertices: &mut [i64],
    ) -> usize {
        let x = xs[v as usize];
        let y = ys[v as usize];

        let nz = self.nz as i64;
        let zx = x as i64 * nz;
        let zy = y as i64 * nz;

        let rad2 = rad * rad;

        let mut num = 0;

        for i in (zx - 1).max(0)..(zx + 2).min(nz) {
            for j in (zy - 1).max(0)..(zy + 2).min(nz) {
                let sz = &self.z[(i * nz + j) as usize];
                for k in 0..sz.count as usize {
                    let l = sz.zv[k];
                    let dx = x - xs[l as usize];
                    let dy = y - ys[l as usize];
                    if dx * dx + dy * dy < rad2 {
                        vertices[num] = l;
                        num += 1;
                    }
                }
            }
        }

        num
    }

    pub(super) fn update_vertex(&mut self, v1: i64, x: f64, y: f64) {
        let old_z = self.vz[v1 as usize];
        if old_z < 0 {
            return;
        }

        let new_z = self.get_z(x, y);

        if new_z != old_z {
            self.remove_vertex_from_zone(old_z, v1);
            self.add_vertex_to_zone(new_z, v1);
            self.vz[v1 as usize] = new_z;
        }
    }
}
