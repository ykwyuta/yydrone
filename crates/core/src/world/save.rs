//! 記録（セーブデータ）の読み書き：外部クレートを使わない小さなバイナリ形式
//!
//! 地図は固定なので作り直し、戦争で変わった部分だけを記録する。弾は飛んでいる途中のものを捨て、
//! 道順は読み込んだあとに引き直す。形式を変えたら SAVE_VERSION を上げる。

use super::*;
use crate::ai::Plan;

const MAGIC: &[u8; 4] = b"YYDR";
pub const SAVE_VERSION: u32 = 5;

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn bool(&mut self, v: bool) {
        self.0.push(v as u8);
    }
    fn u32(&mut self, mut v: u32) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.0.push(b);
                return;
            }
            self.0.push(b | 0x80);
        }
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn pair(&mut self, p: (f32, f32)) {
        self.f32(p.0);
        self.f32(p.1);
    }
    fn opt_pair(&mut self, p: Option<(f32, f32)>) {
        self.bool(p.is_some());
        if let Some(p) = p {
            self.pair(p);
        }
    }
    /// 同じ値の並びを圧縮して書く
    fn rle(&mut self, data: &[u8]) {
        let mut i = 0;
        while i < data.len() {
            let v = data[i];
            let mut n = 1;
            while i + n < data.len() && data[i + n] == v {
                n += 1;
            }
            self.u32(n as u32);
            self.u8(v);
            i += n;
        }
    }
    fn target(&mut self, t: Target) {
        match t {
            Target::Unit(id) => {
                self.u8(0);
                self.u32(id);
            }
            Target::Bld(id) => {
                self.u8(1);
                self.u32(id);
            }
        }
    }
    fn order(&mut self, o: Order) {
        match o {
            Order::Idle => self.u8(0),
            Order::Move(x, y) => {
                self.u8(1);
                self.pair((x, y));
            }
            Order::AttackMove(x, y) => {
                self.u8(2);
                self.pair((x, y));
            }
            Order::Attack(t) => {
                self.u8(3);
                self.target(t);
            }
            Order::AttackGround(x, y) => {
                self.u8(4);
                self.pair((x, y));
            }
            Order::Hold => self.u8(5),
        }
    }
    fn mod_kind(&mut self, k: ModKind) {
        let (tag, v) = match k {
            ModKind::Protect(v) => (0, v),
            ModKind::Slow(v) => (1, v),
            ModKind::NoSupply => (2, 0.0),
            ModKind::Outage => (3, 0.0),
            ModKind::Uplink => (4, 0.0),
            ModKind::IncomeMul(v) => (5, v),
            ModKind::ShellMul(v) => (6, v),
            ModKind::Frozen => (7, 0.0),
        };
        self.u8(tag);
        self.f32(v);
    }
}

struct Reader<'a> {
    d: &'a [u8],
    p: usize,
}

type R<T> = Result<T, String>;
const BAD: &str = "記録が壊れている";

impl Reader<'_> {
    fn u8(&mut self) -> R<u8> {
        let v = *self.d.get(self.p).ok_or(BAD)?;
        self.p += 1;
        Ok(v)
    }
    fn bool(&mut self) -> R<bool> {
        Ok(self.u8()? != 0)
    }
    fn u32(&mut self) -> R<u32> {
        let (mut v, mut s) = (0u32, 0);
        loop {
            let b = self.u8()?;
            if s >= 32 {
                return Err(BAD.into());
            }
            v |= ((b & 0x7f) as u32) << s;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            s += 7;
        }
    }
    fn usize(&mut self) -> R<usize> {
        Ok(self.u32()? as usize)
    }
    fn take(&mut self, n: usize) -> R<&[u8]> {
        let e = self.p.checked_add(n).ok_or(BAD)?;
        let s = self.d.get(self.p..e).ok_or(BAD)?;
        self.p = e;
        Ok(s)
    }
    fn u64(&mut self) -> R<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(|_| BAD)?))
    }
    fn f32(&mut self) -> R<f32> {
        let v = f32::from_le_bytes(self.take(4)?.try_into().map_err(|_| BAD)?);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(BAD.into())
        }
    }
    fn pair(&mut self) -> R<(f32, f32)> {
        Ok((self.f32()?, self.f32()?))
    }
    fn opt_pair(&mut self) -> R<Option<(f32, f32)>> {
        Ok(if self.bool()? { Some(self.pair()?) } else { None })
    }
    fn rle(&mut self, out: &mut [u8]) -> R<()> {
        let mut i = 0;
        while i < out.len() {
            let n = self.usize()?;
            let v = self.u8()?;
            if n == 0 || i + n > out.len() {
                return Err(BAD.into());
            }
            out[i..i + n].fill(v);
            i += n;
        }
        Ok(())
    }
    fn side(&mut self) -> R<u8> {
        let s = self.u8()?;
        if s < 2 {
            Ok(s)
        } else {
            Err(BAD.into())
        }
    }
    fn kind(&mut self) -> R<UK> {
        Ok(DEFS.get(self.u8()? as usize).ok_or(BAD)?.kind)
    }
    fn target(&mut self) -> R<Target> {
        Ok(match self.u8()? {
            0 => Target::Unit(self.u32()?),
            1 => Target::Bld(self.u32()?),
            _ => return Err(BAD.into()),
        })
    }
    fn order(&mut self) -> R<Order> {
        Ok(match self.u8()? {
            0 => Order::Idle,
            1 => {
                let p = self.pair()?;
                Order::Move(p.0, p.1)
            }
            2 => {
                let p = self.pair()?;
                Order::AttackMove(p.0, p.1)
            }
            3 => Order::Attack(self.target()?),
            4 => {
                let p = self.pair()?;
                Order::AttackGround(p.0, p.1)
            }
            5 => Order::Hold,
            _ => return Err(BAD.into()),
        })
    }
    fn mod_kind(&mut self) -> R<ModKind> {
        let tag = self.u8()?;
        let v = self.f32()?;
        Ok(match tag {
            0 => ModKind::Protect(v),
            1 => ModKind::Slow(v),
            2 => ModKind::NoSupply,
            3 => ModKind::Outage,
            4 => ModKind::Uplink,
            5 => ModKind::IncomeMul(v),
            6 => ModKind::ShellMul(v),
            7 => ModKind::Frozen,
            _ => return Err(BAD.into()),
        })
    }
}

fn terrain_code(t: Terrain) -> u8 {
    t as u8
}

fn terrain_from(c: u8) -> R<Terrain> {
    Ok(match c {
        0 => Terrain::Sea,
        1 => Terrain::River,
        2 => Terrain::Plain,
        3 => Terrain::Forest,
        4 => Terrain::Urban,
        5 => Terrain::Bridge,
        6 => Terrain::Neutral,
        7 => Terrain::Marsh,
        8 => Terrain::Treeline,
        9 => Terrain::Hill,
        _ => return Err(BAD.into()),
    })
}

fn job_code(k: JobKind) -> u8 {
    match k {
        JobKind::Trench => 0,
        JobKind::Mine => 1,
        JobKind::Teeth => 2,
        JobKind::Pontoon => 3,
    }
}

fn job_from(c: u8) -> R<JobKind> {
    Ok(match c {
        0 => JobKind::Trench,
        1 => JobKind::Mine,
        2 => JobKind::Teeth,
        3 => JobKind::Pontoon,
        _ => return Err(BAD.into()),
    })
}

impl World {
    pub fn save_bytes(&self) -> Vec<u8> {
        let mut w = Writer(Vec::with_capacity(1 << 17));
        w.0.extend_from_slice(MAGIC);
        w.u32(SAVE_VERSION);
        w.f32(self.t);
        w.u64(self.ticks);
        w.f32(self.duration);
        w.u64(self.rng.0);
        w.u8(self.player.map_or(2, |p| p));
        // タイル
        w.rle(&self.owner.iter().map(|o| *o as u8).collect::<Vec<_>>());
        w.rle(&self.feat);
        w.rle(&self.mines);
        w.rle(&self.map.terrain.iter().map(|t| terrain_code(*t)).collect::<Vec<_>>());
        w.rle(&self.map.road.iter().map(|r| *r as u8).collect::<Vec<_>>());
        w.rle(&self.map.rail.iter().map(|r| *r as u8).collect::<Vec<_>>());
        w.rle(&self.cap.iter().map(|c| (*c).clamp(0, 255) as u8).collect::<Vec<_>>());
        // 橋（最初からある橋は耐久だけ、浮橋は場所も）
        w.u32(self.map.bridges.len() as u32);
        for b in &self.map.bridges {
            w.f32(b.hp);
        }
        let base = Map::base_bridge_count(&self.map);
        for b in &self.map.bridges[base..] {
            w.u32(b.tiles[0]);
        }
        for c in &self.map.cities {
            w.u8(c.owner);
        }
        // 陣営
        for s in &self.sides {
            w.f32(s.funds);
            w.f32(s.shells);
            w.u32(s.kills);
            w.u32(s.losses);
            for c in s.power_cd {
                w.f32(c);
            }
            w.bool(s.ai);
            w.f32(s.ai_mult);
            w.bool(s.auto_new);
            w.bool(s.auto_buy);
            w.u32(s.plan.axes.len() as u32);
            for a in &s.plan.axes {
                w.u32(*a as u32);
            }
            w.f32(s.plan.axis_t);
            w.f32(s.plan.build_t);
            w.f32(s.plan.buy_t);
            w.f32(s.plan.power_t);
            w.u32(s.plan.forced.len() as u32);
            for a in &s.plan.forced {
                w.u32(*a as u32);
            }
            w.u32(s.plan.forced_until);
        }
        // 部隊
        w.u32(self.id_map.len() as u32);
        w.u32(self.units.len() as u32);
        for u in &self.units {
            w.u32(u.id);
            w.u8(u.kind as u8);
            w.u8(u.side);
            w.pair((u.x, u.y));
            w.f32(u.hp);
            w.f32(u.face);
            w.order(u.order);
            w.opt_pair(u.goal);
            w.bool(u.target.is_some());
            if let Some(t) = u.target {
                w.target(t);
            }
            w.f32(u.cd);
            w.f32(u.still);
            w.f32(u.dig);
            w.bool(u.auto);
            w.f32(u.life);
            w.u32(u.jobs.len() as u32);
            for j in &u.jobs {
                w.u8(job_code(j.kind));
                w.u32(j.x as u32);
                w.u32(j.y as u32);
            }
            w.f32(u.job_t);
            w.u8(u.sortie);
            w.pair(u.home);
            w.f32(u.flash);
            w.bool(u.supplied);
            w.f32(u.ai_t);
        }
        // 建物
        w.u32(self.bld_seq);
        w.u32(self.blds.len() as u32);
        for b in &self.blds {
            w.u32(b.id);
            w.u8(b.kind as u8);
            w.u8(b.side);
            w.pair((b.x, b.y));
            w.f32(b.hp);
            w.f32(b.building);
            w.u32(b.queue.len() as u32);
            for q in &b.queue {
                w.u8(*q as u8);
            }
            w.f32(b.prog);
            w.opt_pair(b.rally);
            w.bool(b.seen[0]);
            w.bool(b.seen[1]);
        }
        // 史実と時限つきの効果
        w.u32(self.next_event as u32);
        for c in &self.chronicle {
            w.bool(c.happened);
        }
        w.u32(self.mods.len() as u32);
        for m in &self.mods {
            w.u8(m.side);
            w.mod_kind(m.kind);
            w.bool(m.zone.is_some());
            if let Some(z) = m.zone {
                w.f32(z.0);
                w.f32(z.1);
                w.f32(z.2);
            }
            w.u32(m.until);
        }
        w.u32(self.halts.len() as u32);
        for h in &self.halts {
            w.u8(h.0);
            w.f32(h.1);
            w.f32(h.2);
            w.f32(h.3);
            w.u32(h.4);
        }
        w.u32(self.flooded.len() as u32);
        for f in &self.flooded {
            w.u32(f.0);
            w.u8(terrain_code(f.1));
        }
        w.u32(self.flood_until);
        w.u32(self.mud_until);
        w.u32(self.rail_cuts.len() as u32);
        for c in &self.rail_cuts {
            w.u32(c.0);
            w.f32(c.1);
        }
        w.u8(self.weather as u8);
        w.f32(self.weather_t);
        // 記録
        w.u32(self.history.len() as u32);
        for h in &self.history {
            w.pair(*h);
        }
        w.u32(self.sats.len() as u32);
        for s in &self.sats {
            w.u8(s.0);
            w.f32(s.1);
            w.f32(s.2);
            w.f32(s.3);
        }
        w.u32(self.scar_log.len() as u32);
        for s in &self.scar_log {
            w.f32(s.0);
            w.f32(s.1);
            w.f32(s.2);
        }
        for side in &self.sides {
            let c=&side.command;
            w.u32(crate::command::GENERAL_COUNT as u32);
            for g in &c.generals {
                w.u8(g.doctrine as u8);w.pair((g.region.x,g.region.y));w.f32(g.region.radius);
                w.f32(g.budget);w.f32(g.spent);w.u32(g.losses);w.f32(g.buy_t);
                w.u32(g.units.len() as u32);for id in &g.units{w.u32(*id);}
            }
            w.u32(c.pending.len() as u32);for p in &c.pending{w.u32(p.building);w.u32(p.slot as u32);w.u8(p.general);}
            w.f32(c.report_t);
            for values in [c.front_losses,c.front_kills,c.last_losses,c.last_kills]{for v in values{w.u32(v);}}
            w.u32(c.reports.len() as u32);for report in &c.reports{w.f32(report.at);w.u8(report.sector as u8);w.u32(report.text.len() as u32);w.0.extend_from_slice(report.text.as_bytes());}
        }
        // 形式4までの本体に続けて、将軍の提案と明示的な地域割当を保存。
        for side in &self.sides {
            let c=&side.command;w.f32(c.proposal_t);
            for (gi,p) in c.proposals.iter().enumerate() {
                w.bool(c.generals[gi].region_assigned);w.f32(c.proposal_after[gi]);w.bool(p.is_some());
                if let Some(p)=p {
                    w.f32(p.at);w.u32(p.city as u32);w.bool(p.region.is_some());
                    if let Some(r)=p.region {w.pair((r.x,r.y));w.f32(r.radius);}
                    w.f32(p.budget);w.u32(p.reason.len() as u32);w.0.extend_from_slice(p.reason.as_bytes());
                }
            }
        }
        w.0
    }

    pub fn load_bytes(data: &[u8]) -> Result<World, String> {
        let mut r = Reader { d: data, p: 0 };
        if r.take(4).ok() != Some(MAGIC.as_slice()) {
            return Err("ドローン戦線の記録ではない".into());
        }
        let ver = r.u32()?;
        if ver != SAVE_VERSION && ver != 4 && ver != 3 {
            return Err(format!("記録の版が違う（記録 {ver} ／ このゲーム {SAVE_VERSION}）"));
        }
        let t = r.f32()?;
        let ticks = r.u64()?;
        let duration = r.f32()?;
        let rng = r.u64()?;
        let player = match r.u8()? {
            2 => None,
            p if p < 2 => Some(p),
            _ => return Err(BAD.into()),
        };
        let mut w = World::new(Settings { duration, seed: 1, ai_mult: 1.0, player });
        // 新しい戦争として始まった分を捨てて、記録の状態に置きかえる
        for side in &mut w.sides {side.command=Default::default();}
        w.units.clear();
        w.blds.clear();
        w.projs.clear();
        w.fx.clear();
        w.notices.clear();
        w.dirty.clear();
        w.repaint.clear();
        w.scars.clear();
        w.pq.clear();
        w.recent.clear();
        w.t = t;
        w.ticks = ticks;
        w.rng.0 = rng;
        let n = W * H;
        let mut buf = vec![0u8; n];
        r.rle(&mut buf)?;
        for (o, b) in w.owner.iter_mut().zip(&buf) {
            *o = *b as i8;
        }
        r.rle(&mut w.feat)?;
        r.rle(&mut w.mines)?;
        r.rle(&mut buf)?;
        for (t, b) in w.map.terrain.iter_mut().zip(&buf) {
            *t = terrain_from(*b)?;
        }
        r.rle(&mut buf)?;
        for (t, b) in w.map.road.iter_mut().zip(&buf) {
            *t = *b != 0;
        }
        r.rle(&mut buf)?;
        for (t, b) in w.map.rail.iter_mut().zip(&buf) {
            *t = *b != 0;
        }
        r.rle(&mut buf)?;
        for (t, b) in w.cap.iter_mut().zip(&buf) {
            *t = *b as i16;
        }
        let bridges = r.usize()?;
        let base = Map::base_bridge_count(&w.map);
        if bridges < base {
            return Err(BAD.into());
        }
        let mut hps = Vec::with_capacity(bridges.min(1 << 16));
        for _ in 0..bridges {
            hps.push(r.f32()?);
        }
        for i in base..bridges {
            let t = r.u32()?;
            if t as usize >= n {
                return Err(BAD.into());
            }
            let (x, y) = ((t as usize % W) as f32 + 0.5, (t as usize / W) as f32 + 0.5);
            w.map.bridges.push(Bridge { tiles: vec![t], hp: hps[i], max_hp: 200.0, under: Terrain::River, x, y, name: Some("浮橋") });
            w.map.bridge_at[t as usize] = i as u16;
        }
        for (b, hp) in w.map.bridges.iter_mut().zip(&hps) {
            b.hp = *hp;
        }
        for c in w.map.cities.iter_mut() {
            c.owner = r.side()?;
        }
        for s in w.sides.iter_mut() {
            s.funds = r.f32()?;
            s.shells = r.f32()?;
            s.kills = r.u32()?;
            s.losses = r.u32()?;
            for c in s.power_cd.iter_mut() {
                *c = r.f32()?;
            }
            s.ai = r.bool()?;
            s.ai_mult = r.f32()?;
            s.auto_new = r.bool()?;
            s.auto_buy = r.bool()?;
            let mut plan = Plan::default();
            for _ in 0..r.usize()? {
                let c = r.usize()?;
                plan.axes.push(c);
            }
            plan.axis_t = r.f32()?;
            plan.build_t = r.f32()?;
            plan.buy_t = r.f32()?;
            plan.power_t = r.f32()?;
            for _ in 0..r.usize()? {
                let c = r.usize()?;
                plan.forced.push(c);
            }
            plan.forced_until = r.u32()?;
            if plan.axes.iter().chain(&plan.forced).any(|c| *c >= w.map.cities.len()) {
                return Err(BAD.into());
            }
            s.plan = plan;
        }
        let next_id = r.usize()?;
        if next_id > 1 << 24 {
            return Err(BAD.into());
        }
        w.id_map = vec![u32::MAX; next_id];
        for _ in 0..r.usize()? {
            let id = r.u32()?;
            if id as usize >= next_id {
                return Err(BAD.into());
            }
            let kind = r.kind()?;
            let side = r.side()?;
            let (x, y) = r.pair()?;
            if !in_map(x as i32, y as i32) {
                return Err(BAD.into());
            }
            let hp = r.f32()?;
            let face = r.f32()?;
            let order = r.order()?;
            let goal = r.opt_pair()?;
            let target = if r.bool()? { Some(r.target()?) } else { None };
            let cd = r.f32()?;
            let still = r.f32()?;
            let dig = r.f32()?;
            let auto = r.bool()?;
            let life = r.f32()?;
            let mut jobs = VecDeque::new();
            for _ in 0..r.usize()? {
                let k = job_from(r.u8()?)?;
                let (jx, jy) = (r.u32()?, r.u32()?);
                if !in_map(jx as i32, jy as i32) {
                    return Err(BAD.into());
                }
                jobs.push_back(Job { kind: k, x: jx as u16, y: jy as u16 });
            }
            let job_t = r.f32()?;
            let sortie = r.u8()?;
            let home = r.pair()?;
            let flash = r.f32()?;
            let supplied = r.bool()?;
            let ai_t = r.f32()?;
            w.id_map[id as usize] = w.units.len() as u32;
            w.units.push(Unit {
                id,
                kind,
                side,
                x,
                y,
                hp,
                face,
                order,
                path: Vec::new(),
                pi: 0,
                goal,
                path_pending: false,
                target,
                cd,
                still,
                dig,
                auto,
                life,
                jobs,
                job_t,
                dead: false,
                stuck: 0,
                sortie,
                home,
                flash,
                supplied,
                ai_t,
                jam_t: 0.0,
                moving: false,
            });
        }
        // 道順は引き直す
        for i in 0..w.units.len() {
            if let Some(g) = w.units[i].goal {
                if def(w.units[i].kind).pass != Pass::Air {
                    w.set_goal(i, g.0, g.1);
                }
            }
        }
        w.bld_seq = r.u32()?;
        for _ in 0..r.usize()? {
            let id = r.u32()?;
            let kind = *BK_ALL.get(r.u8()? as usize).ok_or(BAD)?;
            let side = r.side()?;
            let (x, y) = r.pair()?;
            if !in_map(x as i32, y as i32) {
                return Err(BAD.into());
            }
            let hp = r.f32()?;
            let building = r.f32()?;
            let mut queue = VecDeque::new();
            for _ in 0..r.usize()? {
                queue.push_back(r.kind()?);
            }
            let prog = r.f32()?;
            let rally = r.opt_pair()?;
            let seen = [r.bool()?, r.bool()?];
            w.blds.push(Bld { id, kind, side, x, y, hp, max_hp: bdef(kind).hp, building, queue, prog, rally, seen, dead: false });
        }
        let next = r.usize()?;
        if next > HISTORY.len() {
            return Err(BAD.into());
        }
        w.next_event = next;
        w.chronicle.clear();
        for e in HISTORY.iter().take(next) {
            let happened = r.bool()?;
            let c = w.chronicle_entry(e, happened);
            w.chronicle.push(c);
        }
        w.mods.clear();
        for _ in 0..r.usize()? {
            let side = r.side()?;
            let kind = r.mod_kind()?;
            let zone = if r.bool()? { Some((r.f32()?, r.f32()?, r.f32()?)) } else { None };
            let until = r.u32()?;
            w.mods.push(Mod { side, kind, zone, until });
        }
        w.halts.clear();
        for _ in 0..r.usize()? {
            w.halts.push((r.side()?, r.f32()?, r.f32()?, r.f32()?, r.u32()?));
        }
        w.flooded.clear();
        for _ in 0..r.usize()? {
            let t = r.u32()?;
            if t as usize >= n {
                return Err(BAD.into());
            }
            w.flooded.push((t, terrain_from(r.u8()?)?));
        }
        w.flood_until = r.u32()?;
        w.mud_until = r.u32()?;
        w.rail_cuts.clear();
        for _ in 0..r.usize()? {
            let t = r.u32()?;
            if t as usize >= n {
                return Err(BAD.into());
            }
            w.rail_cuts.push((t, r.f32()?));
        }
        w.weather = match r.u8()? {
            0 => Weather::Clear,
            1 => Weather::Rain,
            2 => Weather::Fog,
            3 => Weather::Snow,
            _ => return Err(BAD.into()),
        };
        w.weather_t = r.f32()?;
        w.history.clear();
        for _ in 0..r.usize()? {
            w.history.push(r.pair()?);
        }
        w.sats.clear();
        for _ in 0..r.usize()? {
            w.sats.push((r.side()?, r.f32()?, r.f32()?, r.f32()?));
        }
        w.scar_log.clear();
        for _ in 0..r.usize()? {
            w.scar_log.push((r.f32()?, r.f32()?, r.f32()?));
        }
        if ver>=4 {
            use crate::command::*;
            for side in 0..2usize {
                let mut c=CommandState::default();let mut assigned=Vec::new();
                if r.usize()?!=GENERAL_COUNT{return Err(BAD.into());}
                for g in &mut c.generals {
                    g.doctrine=match r.u8()?{0=>Doctrine::Assault,1=>Doctrine::Defend,2=>Doctrine::Flexible,_=>return Err(BAD.into())};
                    let (x,y)=r.pair()?;let radius=r.f32()?;
                    if x<0.0||x>=W as f32||y<0.0||y>=H as f32||!(4.0..=160.0).contains(&radius){return Err(BAD.into());}
                    g.region=Region{x,y,radius};g.budget=r.f32()?;g.spent=r.f32()?;g.losses=r.u32()?;g.buy_t=r.f32()?;
                    if g.budget<0.0||g.spent<0.0{return Err(BAD.into());}
                    let count=r.usize()?;if count>w.units.len(){return Err(BAD.into());}
                    for _ in 0..count {let id=r.u32()?;if assigned.contains(&id)||!w.units.iter().any(|u|u.id==id&&u.side as usize==side&&u.auto){return Err(BAD.into());}g.units.push(id);assigned.push(id);}
                }
                let count=r.usize()?;if count>w.blds.iter().map(|b|b.queue.len()).sum(){return Err(BAD.into());}
                for _ in 0..count {
                    let building=r.u32()?;let slot=r.usize()?;let general=r.u8()?;
                    if general>DIRECT_COMMAND||!w.blds.iter().any(|b|b.id==building&&b.side as usize==side&&slot<b.queue.len())||c.pending.iter().any(|p|p.building==building&&p.slot==slot){return Err(BAD.into());}
                    c.pending.push(PendingRecruit{building,slot,general});
                }
                c.report_t=r.f32()?;
                for values in [&mut c.front_losses,&mut c.front_kills,&mut c.last_losses,&mut c.last_kills]{for v in values{*v=r.u32()?;}}
                let count=r.usize()?;if count>18{return Err(BAD.into());}
                for _ in 0..count {let at=r.f32()?;let sector=r.u8()? as usize;let len=r.usize()?;if sector>=3||len>4096{return Err(BAD.into());}let text=String::from_utf8(r.take(len)?.to_vec()).map_err(|_|BAD)?;c.reports.push_back(FrontReport{at,sector,text});}
                w.sides[side].command=c;
            }
        }
        if ver>=5 {
            use crate::command::*;
            for side in 0..2 {
                let c=&mut w.sides[side].command;c.proposal_t=r.f32()?;
                if !(0.0..=90.0).contains(&c.proposal_t){return Err(BAD.into());}
                for gi in 0..GENERAL_COUNT {
                    c.generals[gi].region_assigned=r.bool()?;c.proposal_after[gi]=r.f32()?;
                    if c.proposal_after[gi]<0.0{return Err(BAD.into());}
                    if r.bool()? {
                        let at=r.f32()?;let city=r.usize()?;
                        if at<0.0||city>=w.map.cities.len(){return Err(BAD.into());}
                        let region=if r.bool()? {
                            let (x,y)=r.pair()?;let radius=r.f32()?;
                            if x<0.0||x>=W as f32||y<0.0||y>=H as f32||!(4.0..=160.0).contains(&radius){return Err(BAD.into());}
                            Some(Region{x,y,radius})
                        }else{None};
                        let budget=r.f32()?;let len=r.usize()?;
                        if budget<0.0||len>4096{return Err(BAD.into());}
                        let reason=String::from_utf8(r.take(len)?.to_vec()).map_err(|_|BAD)?;
                        c.proposals[gi]=Some(GeneralProposal{at,city,region,budget,reason});
                    }
                }
            }
        }
        if r.p!=data.len(){return Err(BAD.into());}
        w.total_tiles = w.owner.iter().filter(|o| **o >= 0).count() as u32;
        w.count_tiles();
        w.notices.clear();
        w.update_season();
        w.notices.clear();
        w.economy(0.0);
        w.update_supply();
        w.update_vision();
        w.owner_rev = w.owner_rev.wrapping_add(1);
        Ok(w)
    }
}
