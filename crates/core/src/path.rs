//! グリッド A* 経路探索（地形の種類ごとの通行コストつき）

use crate::geo::{tidx, in_map, Map, Pass, Terrain, H, W};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Copy, Clone)]
struct Node {
    f: f32,
    idx: u32,
}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}

const DIRS: [(i32, i32, f32); 8] = [(1, 0, 1.0), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0), (1, 1, 1.41421), (1, -1, 1.41421), (-1, 1, 1.41421), (-1, -1, 1.41421)];

pub struct PathFinder {
    pub last_exp: usize,
    g: Vec<f32>,
    parent: Vec<u32>,
    seen: Vec<u32>,
    closed: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Node>,
}

impl PathFinder {
    pub fn new() -> Self {
        let n = W * H;
        PathFinder { last_exp: 0, g: vec![0.0; n], parent: vec![0; n], seen: vec![0; n], closed: vec![0; n], stamp: 0, heap: BinaryHeap::new() }
    }

    /// 道順を探す。feat はタイルの地物（竜の歯は車両が通れない）。始点からの点列を返す
    pub fn find(&mut self, map: &Map, feat: &[u8], pass: Pass, sx: i32, sy: i32, gx: i32, gy: i32, weight: f32, max_exp: usize) -> Option<Vec<(u16, u16)>> {
        self.stamp += 1;
        self.heap.clear();
        self.last_exp = 0;
        let (gx, gy) = nearest_passable(map, feat, pass, gx, gy)?;
        let start = tidx(sx, sy);
        let goal = tidx(gx, gy);
        if start == goal {
            return Some(vec![(gx as u16, gy as u16)]);
        }
        let stamp = self.stamp;
        self.g[start] = 0.0;
        self.seen[start] = stamp;
        self.parent[start] = u32::MAX;
        let h = |x: i32, y: i32| {
            let (dx, dy) = ((x - gx).abs() as f32, (y - gy).abs() as f32);
            (dx.max(dy) + 0.41421 * dx.min(dy)) * weight
        };
        self.heap.push(Node { f: h(sx, sy), idx: start as u32 });
        while let Some(n) = self.heap.pop() {
            let i = n.idx as usize;
            if self.closed[i] == stamp {
                continue;
            }
            self.closed[i] = stamp;
            self.last_exp += 1;
            if i == goal {
                let mut out = Vec::new();
                let mut c = i as u32;
                while c != u32::MAX {
                    out.push(((c as usize % W) as u16, (c as usize / W) as u16));
                    c = self.parent[c as usize];
                }
                out.reverse();
                return Some(out);
            }
            if self.last_exp > max_exp {
                return None;
            }
            let (cx, cy) = ((i % W) as i32, (i / W) as i32);
            let gc = self.g[i];
            for (dx, dy, dc) in DIRS {
                let (nx, ny) = (cx + dx, cy + dy);
                if !in_map(nx, ny) {
                    continue;
                }
                let ni = tidx(nx, ny);
                if self.closed[ni] == stamp {
                    continue;
                }
                let Some(c) = tile_cost(map, feat, ni, pass) else { continue };
                if dx != 0 && dy != 0 && (tile_cost(map, feat, tidx(cx + dx, cy), pass).is_none() || tile_cost(map, feat, tidx(cx, cy + dy), pass).is_none()) {
                    continue;
                }
                let ng = gc + dc * c;
                if self.seen[ni] != stamp || ng < self.g[ni] {
                    self.seen[ni] = stamp;
                    self.g[ni] = ng;
                    self.parent[ni] = i as u32;
                    self.heap.push(Node { f: ng + h(nx, ny), idx: ni as u32 });
                }
            }
        }
        None
    }
}

pub const FEAT_TRENCH: u8 = 1;
pub const FEAT_TEETH: u8 = 2;

#[inline]
pub fn tile_cost(map: &Map, feat: &[u8], i: usize, pass: Pass) -> Option<f32> {
    let c = map.cost(i, pass)?;
    if pass == Pass::Veh && feat[i] & FEAT_TEETH != 0 {
        return None;
    }
    if pass != Pass::Sea && map.terrain[i] == Terrain::Bridge {
        return Some(c);
    }
    Some(c)
}

/// 目的地が通れない地形なら、いちばん近い通れるタイルに置きかえる
pub fn nearest_passable(map: &Map, feat: &[u8], pass: Pass, x: i32, y: i32) -> Option<(i32, i32)> {
    let (x, y) = (x.clamp(0, W as i32 - 1), y.clamp(0, H as i32 - 1));
    if tile_cost(map, feat, tidx(x, y), pass).is_some() {
        return Some((x, y));
    }
    for r in 1..40i32 {
        for d in -r..=r {
            for (px, py) in [(x + d, y - r), (x + d, y + r), (x - r, y + d), (x + r, y + d)] {
                if in_map(px, py) && tile_cost(map, feat, tidx(px, py), pass).is_some() {
                    return Some((px, py));
                }
            }
        }
    }
    None
}
