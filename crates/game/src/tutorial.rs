//! 独立した訓練場で実際の操作を学ぶ。進行・保存は通常戦と分離する。
use crate::gfx::*;
use crate::ui::{TextQ, ACCENT, DIM, LINE, PANEL, TEXT};
use crate::{Game, Mode, Screen};
use yydrone_core::command::Doctrine;
use yydrone_core::defs::*;
use yydrone_core::world::*;

pub(crate) struct TutorialState {
    pub lesson: TrainingLesson,
    pub actors: TrainingActors,
    baseline: u32,
    pub ready: bool,
    origin: (f32, f32),
    zoom: f32,
    panned: bool,
    zoomed: bool,
    pub results: [u8; TRAINING_TASKS],
}

fn lesson_text(l: TrainingLesson) -> (&'static str, &'static str, &'static str) {
    use TrainingLesson::*;
    match l {
        Welcome => ("訓練へようこそ", "13の課題で、戦場を見るところから司令部の使い方まで練習します。", "操作を達成すると「次へ」が点灯。やり直し・スキップ・終了も自由です。\n訓練は記録されず、通常作戦のセーブには影響しません。"),
        Camera => ("戦場を見渡す", "カメラを移動し、拡大・縮小してください。", "矢印キー／画面端／中ボタンドラッグで移動、ホイールで拡大・縮小。\nタッチは一本指で地図をなぞって移動、二本指で拡大・縮小します。"),
        Select => ("部隊を選ぶ", "黄色い輪の歩兵を選択してください。", "歩兵を左クリック／タップ。ドラッグで複数選択もできます。\n自軍を選ぶと時間が止まり、落ち着いて指示を出せます。"),
        Move => ("移動を命じる", "歩兵を黄色い目標地点まで移動させてください。", "歩兵を選び、目標地点を右クリック。命令すると時間が進みます。\nタッチは「移動」を押して目的地をタップ。Q は交戦しない移動です。"),
        Capture => ("拠点を制圧する", "歩兵を訓練拠点へ送り、自軍の支配にしてください。", "歩兵を選び、黄色い拠点へ移動命令を出します。近くに留まると制圧。\n塗り替えには時間が必要です。×4 で時間を速められます。"),
        Combat => ("敵を攻撃する", "戦車を前進させ、訓練用の敵IFVを撃破してください。", "戦車を選び、黄色い敵の近くへ右クリック。射程内で自動射撃します。\nこの訓練の敵は反撃しません。距離・視界・兵器の相性が大切です。"),
        Production => ("部隊を生産する", "新しい歩兵を1部隊、生産完了まで待ってください。", "画面下の「地上」から「歩兵」を選ぶと、基地の生産キューに入ります。\nタッチは「生産」から。選択で停止中なら選択解除して時間を進めます。"),
        Construction => ("補給所を建設する", "自国の領土に補給所を置き、完成まで待ってください。", "「建設」→「補給所」を選び、黄色い目印付近の空き地を指定します。\n建物から少し離して配置。補給は部隊の継戦能力を支えます。"),
        ReconProduction => ("偵察ドローンを生産する", "新しい偵察ドローンを1機、生産してください。", "「ドローン」→「偵察機」を選び、生産完了を待ちます。\n工房は設置済み。ドローンも資金と生産時間を使います。"),
        ReconMove => ("偵察する", "偵察ドローンを黄色い偵察地点まで移動させてください。", "工房付近の偵察ドローンを選び、黄色い目標へ移動命令を出します。\n霧の中では敵が見えません。先に偵察し、見えた敵を火力で叩きます。"),
        Fpv => ("FPVで車両を攻撃する", "FPV班を生産・前進させ、訓練用の敵戦車を撃破してください。", "「地上」→「FPV班」を生産し、前進地点へ右クリックで移動。\n偵察機は配置済み。敵を視認し射程に入るとFPVを自動発進します。"),
        Delegate => ("将軍に指揮を委任する", "部隊・予算100・訓練場を含む地域を委任し、「全力防御」を指定。", "歩兵を選び「司令部」→将軍→「地域と部隊」で割当と地域の中心指定。\n「指示と予算」で全力防御と予算+100。特性によって戦い方が変わります。"),
        Staff => ("参謀の助言を読む", "「司令部」→「参謀本部」→「助言と戦力案」を開いてください。", "参謀は確認済みの敵と自軍を見て、行動と不足戦力を提案します。\n戦力案の発注は任意。部隊を直接指揮するときの判断材料にできます。"),
        Report => ("戦線レポートを読む", "時間を進め、参謀本部の「戦線レポート」で報告を確認してください。", "司令部を閉じ、部隊の選択も解除して待ちます。初回は約8秒で届きます。\n通知が来たら「司令部」→「参謀本部」→「戦線レポート」を開きます。"),
        Complete => ("訓練終了", "通常作戦では偵察・補給・増援を組み合わせ、戦線を維持しましょう。", "困ったら「？」で操作を確認し、参謀本部の助言と定期報告を活用。\n練習は何度でもやり直せます。通常作戦はタイトルの設定で開始します。"),
    }
}

impl Game {
    pub(crate) fn start_tutorial(&mut self, g: &mut dyn Gfx) {
        self.training_checkpoint(g, TrainingLesson::Welcome, [0; TRAINING_TASKS]);
    }
    fn training_checkpoint(
        &mut self,
        g: &mut dyn Gfx,
        lesson: TrainingLesson,
        results: [u8; TRAINING_TASKS],
    ) {
        let side = self.cfg_side.clamp(0, 1) as u8;
        let (w, actors) = World::training(side, lesson);
        let baseline = w.units.iter().map(|u| u.id).max().unwrap_or(0);
        self.begin_world(g, w, Some(side));
        self.show_brief = false;
        let (sw, sh) = g.size();
        self.cam = (108.0, 94.0);
        self.zoom = (sw / 90.0)
            .min((sh - self.top_h() - self.training_bottom()) / 42.0)
            .max(4.0);
        self.tutorial = Some(TutorialState {
            lesson,
            actors,
            baseline,
            ready: matches!(lesson, TrainingLesson::Welcome | TrainingLesson::Complete),
            origin: self.cam,
            zoom: self.zoom,
            panned: false,
            zoomed: false,
            results,
        });
    }
    fn training_bottom(&self) -> f32 {
        if self.compact {
            120.0
        } else {
            190.0
        }
    }
    pub(crate) fn tutorial_bookend(&self) -> bool {
        self.tutorial
            .as_ref()
            .is_some_and(|t| matches!(t.lesson, TrainingLesson::Welcome | TrainingLesson::Complete))
    }
    pub(crate) fn update_tutorial(&mut self) {
        let Some(t) = self.tutorial.as_mut() else {
            return;
        };
        let Some(w) = self.world.as_ref() else { return };
        let side = self.viewer.unwrap_or(0);
        let near = |id: Option<u32>, x: f32, y: f32| {
            id.and_then(|id| w.unit(id))
                .is_some_and(|u| (u.x - x).hypot(u.y - y) < 2.5)
        };
        let produced = |kind| {
            w.units
                .iter()
                .any(|u| u.side == side && u.id > t.baseline && u.kind == kind)
        };
        use TrainingLesson::*;
        t.panned |= (self.cam.0 - t.origin.0).hypot(self.cam.1 - t.origin.1) > 1.0;
        t.zoomed |= (self.zoom - t.zoom).abs() > 0.15;
        t.ready |= match t.lesson {
            Welcome | Complete => true,
            Camera => t.panned && t.zoomed,
            Select => self.selected.contains(&t.actors.infantry),
            Move => near(Some(t.actors.infantry), 98.0, 100.0),
            Capture => w.map.cities[0].owner == side,
            Combat => t.actors.target.is_some_and(|id| w.unit(id).is_none()),
            Production => produced(UK::Infantry),
            Construction => w
                .blds
                .iter()
                .any(|b| b.side == side && b.kind == BK::Depot && b.building <= 0.0 && !b.dead),
            ReconProduction => produced(UK::ReconDrone),
            ReconMove => near(t.actors.recon, 126.0, 86.0),
            Fpv => {
                w.training_fpv_launches > 0
                    && t.actors.target.is_some_and(|id| w.unit(id).is_none())
            }
            Delegate => w.sides[side as usize]
                .command
                .generals
                .iter()
                .any(|general| {
                    general.doctrine == Doctrine::Defend
                        && general.budget + general.spent >= 99.9
                        && general.region.contains(108.0, 94.0)
                        && general
                            .units
                            .iter()
                            .any(|id| w.unit(*id).is_some_and(|u| u.side == side))
                }),
            Staff => self.show_command && self.command_tab == 1 && self.command_staff_tab == 0,
            Report => {
                w.sides[side as usize].command.reports.len() >= 3
                    && self.show_command
                    && self.command_tab == 1
                    && self.command_staff_tab == 1
            }
        };
    }
    pub(crate) fn tutorial_next(&mut self, g: &mut dyn Gfx, skip: bool) {
        let Some(t) = self.tutorial.as_ref() else {
            return;
        };
        if !skip && !t.ready {
            return;
        }
        let index = t.lesson as usize;
        if index >= TRAINING_LESSONS.len() - 1 {
            return;
        }
        let mut results = t.results;
        if index > 0 {
            results[index - 1] = if skip { 2 } else { 1 };
        }
        self.training_checkpoint(g, TRAINING_LESSONS[index + 1], results);
    }
    fn exit_tutorial(&mut self) {
        self.tutorial = None;
        self.world = None;
        self.screen = Screen::Title;
        self.show_command = false;
        self.show_save = false;
        self.show_help = false;
        self.show_chron = false;
        self.selected.clear();
        self.mode = Mode::Normal;
        self.pressed = None;
    }
    fn training_goal(&self) -> Option<(f32, f32, &'static str)> {
        let t = self.tutorial.as_ref()?;
        use TrainingLesson::*;
        match t.lesson {
            Select => self
                .world
                .as_ref()?
                .unit(t.actors.infantry)
                .map(|u| (u.x, u.y, "この歩兵を選択")),
            Move => Some((98.0, 100.0, "移動目標")),
            Capture => Some((110.0, 100.0, "制圧目標")),
            Combat => Some((120.0, 100.0, "敵IFV（反撃なし）")),
            Construction => Some((94.0, 106.0, "補給所の建設候補地")),
            ReconMove => Some((126.0, 86.0, "偵察目標")),
            Fpv => Some((108.0, 104.0, "FPV班の前進地点")),
            Delegate => Some((108.0, 94.0, "担当地域の中心")),
            _ => None,
        }
    }
    pub(crate) fn draw_training_save_notice(
        &mut self,
        g: &mut dyn Gfx,
        tq: &mut TextQ,
        sw: f32,
        sh: f32,
    ) {
        let x = sw / 2.0 - 250.0;
        let y = sh / 2.0 - 80.0;
        g.rect(0.0, 0.0, sw, sh, [0.0, 0.0, 0.0, 0.8]);
        self.bg_rects.push((0.0, 0.0, sw, sh));
        tq.add(
            "訓練は保存されません",
            x,
            y,
            500.0,
            32.0,
            Font::BigCenter,
            TEXT,
        );
        tq.add(
            "通常作戦のセーブはそのまま残ります。\n各課題は「やり直す」で最初の状態に戻せます。",
            x,
            y + 40.0,
            500.0,
            50.0,
            Font::NormalCenter,
            DIM,
        );
        if self
            .button(
                g,
                tq,
                "訓練へ戻る",
                x + 150.0,
                y + 100.0,
                200.0,
                36.0,
                true,
                true,
            )
            .is_some()
        {
            self.show_save = false;
        }
    }
    pub(crate) fn draw_tutorial(&mut self, g: &mut dyn Gfx, tq: &mut TextQ) {
        let Some(t) = self.tutorial.as_ref() else {
            return;
        };
        let lesson = t.lesson;
        let ready = t.ready;
        let results = t.results;
        let (title, goal, hint) = lesson_text(lesson);
        let (sw, sh) = g.size();
        if self.modal_open() {
            g.rect(0.0, sh - 26.0, sw, sh, PANEL);
            tq.add(
                format!(
                    "訓練 {}/13：{}　｜　閉じるとガイドへ戻ります",
                    (lesson as usize).min(13),
                    title
                ),
                12.0,
                sh - 25.0,
                sw - 24.0,
                24.0,
                Font::Small,
                ACCENT,
            );
            return;
        }
        let (x, y, width, height) = (
            12.0,
            self.top_h() + 8.0,
            if self.compact { 520.0 } else { 590.0 },
            196.0,
        );
        let marker = self.training_goal();
        if let Some((wx, wy, label)) = marker {
            let (sx, sy) = self.world_to_screen(g, wx, wy);
            if sy > self.top_h() + 20.0
                && sy < sh - self.training_bottom() - 16.0
                && sx > 20.0
                && sx < sw - 20.0
                && !(sx < x + width + 20.0 && sy < y + height + 20.0)
            {
                g.ring(sx, sy, (self.zoom * 1.5).max(14.0), 2.0, 40, ACCENT);
                tq.shadow(
                    label,
                    sx - 110.0,
                    sy + 18.0,
                    220.0,
                    22.0,
                    Font::NormalCenter,
                    ACCENT,
                );
            }
        }
        // 文字は別レイヤーなので、ガイドの下の通知・ラベルも隠す。
        tq.0.retain(|(_, tx, ty, tw, th, _, _, _)| {
            *tx + *tw <= x || *tx >= x + width || *ty + *th <= y || *ty >= y + height
        });
        g.rect(x, y, x + width, y + height, PANEL);
        g.rect_outline(x, y, x + width, y + height, 1.0, LINE);
        self.bg_rects.push((x, y, x + width, y + height));
        let number = lesson as usize;
        tq.add(
            if number == 0 {
                format!("チュートリアル — {title}")
            } else if lesson == TrainingLesson::Complete {
                title.into()
            } else {
                format!("訓練 {number}/13 — {title}")
            },
            x + 12.0,
            y + 6.0,
            width - 24.0,
            27.0,
            Font::Normal,
            ACCENT,
        );
        tq.add(
            goal,
            x + 12.0,
            y + 38.0,
            width - 24.0,
            37.0,
            Font::Normal,
            TEXT,
        );
        tq.add(
            if self.touch {hint.replace("右クリック", "タップ")} else {hint.into()},
            x + 12.0,
            y + 78.0,
            width - 24.0,
            55.0,
            Font::Small,
            DIM,
        );
        let status = if lesson == TrainingLesson::Complete {
            format!(
                "達成 {} / スキップ {}",
                results.iter().filter(|v| **v == 1).count(),
                results.iter().filter(|v| **v == 2).count()
            )
        } else if lesson == TrainingLesson::Welcome {
            "通常戦から独立した専用の訓練場です".into()
        } else if ready {
            "✓ 達成！ 次の課題へ進めます".into()
        } else {
            "操作して目標を達成しましょう（時間短縮は ×4）".into()
        };
        let holding = self.cmd_hold() || self.paused;
        tq.add(
            status,
            x + 12.0,
            y + 134.0,
            width - if holding { 168.0 } else { 24.0 },
            22.0,
            Font::Small,
            if ready { ACCENT } else { DIM },
        );
        if holding
            && self
                .button(
                    g,
                    tq,
                    "選択解除・再開",
                    x + width - 148.0,
                    y + 130.0,
                    136.0,
                    26.0,
                    false,
                    true,
                )
                .is_some()
        {
            self.selected.clear();
            self.sel_bld = None;
            self.cmd_pause = false;
            self.paused = false;
            self.mode = Mode::Normal;
        }
        let by = y + 160.0;
        if lesson == TrainingLesson::Complete {
            if self
                .button(g, tq, "通常作戦へ", x + 12.0, by, 136.0, 30.0, true, true)
                .is_some()
            {
                self.start_game(g);
                return;
            }
            if self
                .button(
                    g,
                    tq,
                    "最初から練習",
                    x + 156.0,
                    by,
                    136.0,
                    30.0,
                    false,
                    true,
                )
                .is_some()
            {
                self.start_tutorial(g);
                return;
            }
            if self
                .button(g, tq, "タイトルへ", x + 300.0, by, 120.0, 30.0, false, true)
                .is_some()
            {
                self.exit_tutorial();
            }
            return;
        }
        if self
            .button(
                g,
                tq,
                if number == 0 {
                    "訓練を始める"
                } else {
                    "次へ"
                },
                x + 12.0,
                by,
                112.0,
                30.0,
                ready,
                ready,
            )
            .is_some()
        {
            self.tutorial_next(g, false);
            return;
        }
        if self
            .button(g, tq, "やり直す", x + 130.0, by, 88.0, 30.0, false, true)
            .is_some()
        {
            self.training_checkpoint(g, lesson, results);
            return;
        }
        if self
            .button(
                g,
                tq,
                "スキップ",
                x + 224.0,
                by,
                88.0,
                30.0,
                false,
                number > 0,
            )
            .is_some()
        {
            self.tutorial_next(g, true);
            return;
        }
        if self
            .button(g, tq, "終了", x + 318.0, by, 66.0, 30.0, false, true)
            .is_some()
        {
            self.exit_tutorial();
            return;
        }
        if marker.is_some()
            && self
                .button(g, tq, "目標へ", x + 390.0, by, 88.0, 30.0, false, true)
                .is_some()
        {
            let (wx, wy, _) = marker.unwrap();
            let sy = ((y + height + 24.0) + (sh - self.training_bottom() - 30.0)) / 2.0;
            self.cam = (
                wx - (sw * 0.65 - sw / 2.0) / self.zoom,
                wy - (sy - sh / 2.0) / self.zoom,
            );
            self.clamp_camera(g);
        }
    }
}
