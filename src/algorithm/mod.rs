mod differential_line;
mod segments;
mod zone_map;

use differential_line::DifferentialLine;

const ONE: f64 = 1. / SIZE as f64;

const N_MAX: u64 = 10_u64.pow(6);
const SIZE: u64 = 1000;

const NEAR_L: f64 = 2. * ONE;
const FAR_L: f64 = 40. * ONE;

const STEP: f64 = 0.4 * ONE;

fn steps(df: &mut DifferentialLine) -> bool {
    df.optimize_position(STEP);

    spawn(df, NEAR_L, 0.001);

    if !df.segments.safe_vertex_positions(3. * STEP) {
        return false;
    }

    true
}

fn spawn(df: &mut DifferentialLine, near_l /* d */: f64, limit: f64) {
    let e_num = df.segments.e_num();

    for e in 0..e_num as i64 {
        let x = 0.1;
        if x < limit {
            let l = df.segments.get_edge_length(e);
            if l < near_l {
                continue;
            }

            // Ignore error
            _ = df.segments.split_edge_no_min(e);
        }
    }
}
