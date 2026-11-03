# 地形・季節・天候・鉄道と、その根拠

実際の戦場で報じられている傾向を、ゲームの効果に置きかえています。数値はゲームとしての調整で、実測値ではありません。

## 地面の状態

| 時期 | 状態 | ゲームでの効果 | 根拠 |
|---|---|---|---|
| 3/15〜4/30、10/10〜11/20 | 泥濘 | 道路の外の速度が装軌 ×0.5、装輪 ×0.28、歩兵 ×0.8。経路探索も道路を選ぶようになる | 道路の外はほぼ通れず、車両は予測しやすい道に集まる [1]。装輪車両は特に難しいと当時の国防相が発言 [2] |
| 12/15〜3/14 | 凍結 | 車両の速度 ×1.15、湿地も走れる。塹壕を掘る速さは 1/4 | 地下水位の高い土地が凍ると機動に最適 [3]。凍った地面は掘れず、壕ひとつに2週間 [4][5] |
| 2022年の開戦時 | 早い泥濘 | 2022年4月末まで泥濘が続く | 暖冬で泥濘が2週間ほど早まった可能性、湿地が固まらなかった [6][7] |

泥濘が実際にどれだけ進撃を遅らせたかについては、資料の間で評価が分かれています [3][6]。

## 葉の有無（4月〜10月は葉がある）

| 場所 | 葉がある時期 | 葉がない時期 | 根拠 |
|---|---|---|---|
| 樹林帯 | 歩兵は動いていても隠れる。車両は止まっていれば隠れる。ドローンでもすぐ近くまで来ないと見つからない | 隠れられない | 冬は葉のない樹林帯と雪で見通しがよく、3月以降は茂った樹林帯を歩兵が動く [8] |
| 森 | 同上 | 止まっていれば地上からは隠れるが、上空からは見える | 同上。落葉で陣地が暴かれる [9] |

樹林帯には歩兵が陣地を置きやすいので、被害も少し減ります [9]。AI の工兵は樹林帯にも地雷を置きます（密な地雷で樹林帯を守るという指摘 [8]）。

## 天候

天候は月ごとの出やすさで決まります。

| 天候 | 地上の視界 | ドローンの視界 | FPV の発射間隔 | 根拠 |
|---|---|---|---|---|
| 雨 | ×0.85 | ×0.6 | ×1.8 | 雨・霧・風でドローンの効果が大きく落ちる [10]。曇天と雨で飛行が減る [11] |
| 霧 | ×0.45 | ×0.35 | ×3.0 | 霧の間にポクロウシクへの浸透が進んだ [10][12] |
| 雪 | ×0.7 | ×0.6 | ×1.6 | 雪と雨で対ドローン防御が弱まる [12] |

悪天候では FPV が墜落しやすくなります。霧はドローンが飛べない分、車両や歩兵を動かす好機になります [11]。ただし霧は両軍に等しく働きます [10]。寒さによる電池の消耗は裏付けを取れなかったので入れていません。

## 地形

| 地形 | 効果 | 根拠 |
|---|---|---|
| 樹林帯 | 畑の間を仕切る細い林。車両 ×0.85、歩兵の被害 ×0.85 | 南部は平坦で開けており、広い畑を樹林帯が仕切る [9] |
| 高地・ボタ山 | 視界 ×1.4、被害が減る、移動が遅い | ボタ山は観測に使われ [13]、攻防の焦点になった [14]。高台は周辺の火力管制に効く [15] |
| 湿地（ポレシア） | 車両 ×0.3、歩兵 ×0.5。建物や塹壕を作れない。凍れば通れる | 湿地が凍らず柔らかいままだった [7] |

ポレシアの広がりは一般的な地理の知識によるもので、今回の調査では戦闘との関係の裏付けを取れていません。ゲーム上のタイルは約5kmあるので、樹林帯やボタ山は実物より大きく表現しています。

## 鉄道

| 仕組み | ゲームでの効果 | 根拠 |
|---|---|---|
| 補給 | つながった自国の線路の上では、補給が距離で弱まらない | ロシア軍の地上作戦の要は鉄道 [16][17]。ウクライナ鉄道は「第二の軍」と呼ばれる [18] |
| 輸送 | 自国の線路上は速度 ×1.9。泥濘の影響を受けない | 大規模な部隊の移動は鉄道に頼る [16][17] |
| 線路の切断 | 爆発で切れ、45秒で復旧する。切れている間は補給が先へ届かない | 列車攻撃で単線の線路が止まった例 [19] |
| 鉄道の要衝 | 橋が落ちると線路も切れる | クリミアへの線路は二本のみで、鉄道橋が狙われる [19][20] |

復旧にかかる時間は裏付けを取れなかったので、45秒はゲームとしての値です。両国の線路は同じ軌間で、国境を越えてつながっている前提で作っています。

## 史実イベントに加えたもの

| 日付 | 見出し | 根拠 |
|---|---|---|
| 2022-02-24 | 暖冬で泥濘が早い | [6][7] |
| 2022-02-25 | イルピア川の氾濫（ダムが破られた経緯は資料で異なる） | [21][22] |
| 2022-02 | 国境の鉄道を破壊 | [16] |
| 2022-07-06 | メリトーポ近くの橋を爆破 | [23] |
| 2025-10 | 鉄道への攻撃が激化 | [24] |
| 2025-11 | 霧の中のポクロヴァ | [10][12] |
| 2026-06 | クリメアへの鉄道橋を攻撃 | [20] |

## 出典

1. [How Ukraine's mud became a secret weapon（NBC）](https://www.nbcnewyork.com/news/business/money-report/how-ukraines-mud-became-a-secret-weapon-in-its-defense-against-russia/3655724/)
2. [Mud slows Ukraine's advance（Boston Globe）](https://www.bostonglobe.com/2022/10/28/world/mud-slows-ukraines-advance-fighting-remains-intense)
3. [Experts are speculating over Russia's next move（CNN／ABC17）](https://abc17news.com/news/national-world/cnn-europe-mideast-africa/2022/02/07/experts-are-speculating-over-russias-next-move-they-may-want-to-check-ukraines-weather-forecast/)
4. [Ukraine and winter warfare（RFE/RL）](https://www.rferl.org/a/ukraine-winter-war-advantage/32166159.html)
5. [Ukrainian soldiers dig trenches near Bakhmut（Deccan Herald）](https://www.deccanherald.com/world/ukrainian-soldiers-dig-trenches-in-case-of-retreat-from-bakhmut-1208759.html)
6. [Ukraine faces severe cold blast（Boston Globe）](https://www.bostonglobe.com/2022/03/09/world/ukraine-faces-severe-cold-blast-it-fights-russian-invasion)
7. [What the onset of 'General Winter' means（Soucy Defense）](https://soucy-defense.com/winter-vehicle-operational-capabilities-ukraine/)
8. [A Ukrainian drone operator explains Russia's spring advantage（Euromaidan Press）](https://euromaidanpress.com/2026/03/05/a-ukrainian-drone-operator-explains-russias-spring-advantage-and-the-two-things-that-can-neutralize-it/)
9. [Ukraine's war of the treelines（CEPA）](https://cepa.org/article/ukraines-war-of-the-treelines/)
10. [The fog of war: changing weather（CNN／AOL）](https://www.aol.com/news/fog-war-changing-weather-becomes-164033660.html)
11. [Winter in Ukraine: when drones can't see, tanks advance（Euromaidan Press）](https://euromaidanpress.com/2025/10/24/winter-is-coming)
12. [Ukraine military situation report, November 26（Hudson Institute）](https://www.hudson.org/national-security-defense/ukraine-military-situation-report-november-26-can-kasapoglu)
13. [Ukrainian army seize spoil heap in Horlivka（RBC-Ukraine）](https://newsukraine.rbc.ua/news/ukrainian-army-seize-spoil-heap-in-horlivka-1702633001.html)
14. [Russia's failed frontal assault on Avdiivka coal spoil tip（Euromaidan Press）](https://euromaidanpress.com/2023/10/16/frontline-report-russias-failed-frontal-assault-on-avdiivka-coal-spoil-tip-shows-russian-strategic-lapses)
15. [Frontline report: coal mine complex near Chasiv Yar（Euromaidan Press）](https://euromaidanpress.com/tags/coal-mine-complex/)
16. [Russia's Military Has a Railroad Problem（Foreign Policy）](https://foreignpolicy.com/2022/04/21/russias-military-has-a-railroad-problem/)
17. [The Iron Leviathan: Russia's rail network in its war against Ukraine（ICDS）](https://icds.ee/en/the-iron-leviathan-russias-rail-network-in-its-war-against-ukraine/)
18. ['Like a second army': Ukraine Railways' key wartime role（VOA）](https://www.voanews.com/a/like-a-second-army-ukraine-railways-key-wartime-role-/6617672.html)
19. [Russian rail logistics crippled after fuel train destroyed（Espreso）](https://global.espreso.tv/russia-ukraine-war-russian-rail-logistics-crippled-after-fuel-train-destroyed-in-occupied-zaporizhzhia)
20. [Ukraine's special forces disabled the rail bridge feeding occupied Crimea（Euromaidan Press）](https://euromaidanpress.com/2026/06/19/ukraines-special-forces-disabled-the-rail-bridge-feeding-russian-occupied-crimea/)
21. [How blowing up a dam in Ukraine flooded a village but stopped Russian forces（NPR）](https://www.npr.org/2022/09/06/1121201310/how-blowing-up-a-dam-in-ukraine-flooded-a-village-but-stopped-russian-forces)
22. [Flood saves Ukrainian village from Russian occupation（AFP／Inquirer）](https://newsinfo.inquirer.net/1598249/flood-saves-ukrainian-village-from-russian-occupation)
23. [Bridge between Melitopol and Tokmak blown up（Ukrainska Pravda）](https://www.pravda.com.ua/eng/news/2022/07/06/7356726/)
24. [Russia strikes Ukraine's rail（RFE/RL）](https://www.rferl.org/a/russia-strikes-ukraine-rail-train-interactive-map/33623690.html)
