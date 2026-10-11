//! Immutable 2D static rectangles. This is a flat-floor authority fixture,
//! not a mesh/capsule/cave/flight collision implementation.
use crate::{Error, Result};
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub min: [i32; 2],
    pub max: [i32; 2],
}
impl Rect {
    fn valid(self) -> bool {
        (0..2).all(|i| self.min[i] < self.max[i])
            && self
                .min
                .iter()
                .chain(&self.max)
                .all(|v| i64::from(*v).abs() <= 1_000_000)
    }
    fn contains(self, p: [i32; 2]) -> bool {
        (0..2).all(|i| self.min[i] <= p[i] && p[i] <= self.max[i])
    }
}
pub struct Scene {
    bounds: Rect,
    obstacles: Vec<Rect>,
}
impl Scene {
    pub fn new(bounds: Rect, obstacles: Vec<Rect>) -> Result<Self> {
        if !bounds.valid()
            || obstacles.len() > 256
            || obstacles.iter().any(|r| {
                !r.valid()
                    || !(0..2).all(|i| bounds.min[i] <= r.min[i] && r.max[i] <= bounds.max[i])
            })
        {
            return Err(Error::InvalidScene);
        }
        Ok(Self { bounds, obstacles })
    }
    pub(crate) fn prepare(self, half_extent: i32) -> Result<Collision> {
        let bounds = Rect {
            min: self.bounds.min.map(|v| v + half_extent),
            max: self.bounds.max.map(|v| v - half_extent),
        };
        if !bounds.valid() {
            return Err(Error::InvalidScene);
        }
        let occluders = self.obstacles.clone();
        let obstacles = self
            .obstacles
            .into_iter()
            .map(|r| Rect {
                min: r.min.map(|v| v - half_extent),
                max: r.max.map(|v| v + half_extent),
            })
            .collect();
        Ok(Collision {
            bounds,
            obstacles,
            occluders,
        })
    }
}
pub(crate) struct Collision {
    bounds: Rect,
    obstacles: Vec<Rect>,
    occluders: Vec<Rect>,
}
impl Collision {
    pub fn line_of_sight(&self, a: [i32; 2], b: [i32; 2]) -> bool {
        !self.occluders.iter().any(|r| segment_intersects(a, b, *r))
    }
    pub fn legal(&self, p: [i32; 2]) -> bool {
        self.bounds.contains(p) && !self.obstacles.iter().any(|r| r.contains(p))
    }
    pub fn allows(&self, start: [i32; 2], end: [i32; 2]) -> bool {
        self.legal(end)
            && !self
                .obstacles
                .iter()
                .any(|r| segment_intersects(start, end, *r))
    }
}
#[derive(Clone, Copy)]
struct Ratio {
    n: i64,
    d: i64,
}
impl Ratio {
    fn new(mut n: i64, mut d: i64) -> Self {
        if d < 0 {
            n = -n;
            d = -d;
        }
        Self { n, d }
    }
    fn less(self, b: Self) -> bool {
        i128::from(self.n) * i128::from(b.d) < i128::from(b.n) * i128::from(self.d)
    }
}
fn segment_intersects(start: [i32; 2], end: [i32; 2], r: Rect) -> bool {
    let mut enter = Ratio::new(0, 1);
    let mut exit = Ratio::new(1, 1);
    for i in 0..2 {
        let delta = i64::from(end[i]) - i64::from(start[i]);
        if delta == 0 {
            if start[i] < r.min[i] || start[i] > r.max[i] {
                return false;
            }
            continue;
        }
        let a = Ratio::new(i64::from(r.min[i]) - i64::from(start[i]), delta);
        let b = Ratio::new(i64::from(r.max[i]) - i64::from(start[i]), delta);
        let (a, b) = if b.less(a) { (b, a) } else { (a, b) };
        if enter.less(a) {
            enter = a;
        }
        if b.less(exit) {
            exit = b;
        }
        if exit.less(enter) {
            return false;
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swept_contacts_and_parallel_segments_are_exact() {
        let r = Rect {
            min: [-10, -10],
            max: [10, 10],
        };
        for (a, b, hit) in [
            ([-100, 0], [100, 0], true),
            ([100, 0], [-100, 0], true),
            ([-100, 11], [100, 11], false),
            ([-100, 10], [100, 10], true),
            ([-100, 100], [100, -100], true),
            ([11, -100], [11, 100], false),
            ([0, 0], [0, 0], true),
            ([100, 100], [100, 100], false),
            ([-20, 0], [-11, 0], false),
        ] {
            assert_eq!(segment_intersects(a, b, r), hit);
        }
    }
    #[test]
    fn randomized_integer_segments_match_independent_orientation_oracle() {
        fn orientation(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> i64 {
            (i64::from(b[0]) - i64::from(a[0])) * (i64::from(c[1]) - i64::from(a[1]))
                - (i64::from(b[1]) - i64::from(a[1])) * (i64::from(c[0]) - i64::from(a[0]))
        }
        fn on(a: [i32; 2], b: [i32; 2], p: [i32; 2]) -> bool {
            orientation(a, b, p) == 0
                && (0..2).all(|i| a[i].min(b[i]) <= p[i] && p[i] <= a[i].max(b[i]))
        }
        fn intersects(a: [i32; 2], b: [i32; 2], c: [i32; 2], d: [i32; 2]) -> bool {
            let v = [
                orientation(a, b, c),
                orientation(a, b, d),
                orientation(c, d, a),
                orientation(c, d, b),
            ];
            on(a, b, c)
                || on(a, b, d)
                || on(c, d, a)
                || on(c, d, b)
                || ((v[0] > 0 && v[1] < 0 || v[0] < 0 && v[1] > 0)
                    && (v[2] > 0 && v[3] < 0 || v[2] < 0 && v[3] > 0))
        }
        let r = Rect {
            min: [-10, -10],
            max: [10, 10],
        };
        let corners = [[-10, -10], [10, -10], [10, 10], [-10, 10]];
        let mut seed = 7919u64;
        let mut coord = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) % 101) as i32 - 50
        };
        for _ in 0..5000 {
            let a = [coord(), coord()];
            let b = [coord(), coord()];
            let expected = r.contains(a)
                || r.contains(b)
                || (0..4).any(|i| intersects(a, b, corners[i], corners[(i + 1) % 4]));
            assert_eq!(segment_intersects(a, b, r), expected);
        }
    }
}
