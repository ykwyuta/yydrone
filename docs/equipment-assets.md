# 兵器・施設の画像

組み込み `image_gen` ツールで31点を個別生成。実在兵器を参考にした架空ゲーム用の外観表現で、寸法・装備を厳密に再現する資料ではありません。ゲームの性能値・生産条件は変更しません。

原画像は `assets/units/equipment_sources/` と `assets/map/equipment_sources/`、実際の画像はそれぞれの親フォルダ内のPNGおよび同名RGBA8ファイルです。兵器は160×160、施設は192×192。背景透過を維持して縮小し、同じ画像を異なる兵種・施設へ割り当てないようにしています。

全プロンプト: [equipment-prompts.json](equipment-prompts.json)。再変換は `python tools/prepare_equipment_assets.py`。生成中の確認には `--available` を付けられます。[生成画像の一覧](equipment-assets-preview.png)。

## 陣営と画像の対応

| ゲーム内の種類 | ウクラヴィア | ロスコヴィア |
| --- | --- | --- |
| 歩兵 | infantry | infantry（共通の汎用歩兵） |
| 工兵 | engineer | engineer（共通の工兵装備） |
| 対戦車班 | atgm_ua（携行式ジャベリン型） | atgm（コルネット型の班） |
| FPV班 | fpv_team | fpv_team（共通の民生由来機材） |
| IFV | ifv（ブラッドレー型） | ifv_ru（BMP-3型） |
| 戦車 | tank_ua（レオパルト2型） | tank（T系戦車型） |
| 自走榴弾砲 | howitzer（M109型） | howitzer_ru（ムスタ型） |
| 多連装ロケット砲 | mlrs_ua（HIMARS型） | mlrs_ru（トルナード型） |
| 沿岸対艦ミサイル | coastal（ネプチューン型） | 生産不可 |
| 近距離対空兵器 | shorad_ua（ゲパルト型） | shorad_ru（パンツィリ型） |
| 長距離SAM | sam_ua（パトリオット型） | sam_ru（S-400型） |
| 電子戦車 | ew_ua（ブコヴェリ型の小型車） | ew_ru（クラスーハ型の大型車） |
| 偵察ドローン | recon_drone_ua（レレカ型） | recon_drone_ru（オルラン型） |
| 爆撃ドローン | bomber_drone_ua（大型6ローター） | bomber_drone_ru（大型8ローター） |
| 武装UAV | armed_uav（TB2型） | armed_uav_ru（オリオン型） |
| 長距離自爆機 | long_drone_ua（リューティ型） | long_drone_ru（シャヘド型） |
| 攻撃ヘリ | heli_ua（Mi-24型） | heli（Ka-52型） |
| 戦闘機 | jet_ua（F-16型） | jet_ru（Su-34型） |
| フリゲート | 生産不可 | frigate（アドミラル・グリゴロヴィチ型） |
| 水上ドローン | seadrone（MAGURA型の無人艇） | 生産不可 |
| FPV飛行機体 | fpv_drone | fpv_drone（共通の小型クアッドコプター） |
| 巡航ミサイル | cruise_ua（ストームシャドウ型） | cruise_ru（カリブル型） |

爆撃ドローンは、両陣営とも既存のゲーム説明どおり繰り返し投弾する大型マルチコプターです。ロスコヴィアの架空名「モルニヤ」は、実在する同名の固定翼機を再現するものではありません。

陣営ごとに型式名の異なる15兵種は別画像。一般歩兵・工兵・FPV班・FPV機体の4種類は、同種の装備を両軍が使用しても自然なため共通。片陣営専用の3種類には利用不可陣営の画像を追加しません。施設は共通構造が自然なので両軍共通ですが、6種類の施設はそれぞれ個別画像です。

## 施設

| 種類 | 画像 | 外観 |
| --- | --- | --- |
| 軍事基地 | military_base | 指揮施設・通信塔 |
| 飛行場 | airfield | 格納庫・管制塔 |
| ドローン工房 | drone_workshop | 小型工房・作業施設 |
| 砲弾工場 | shell_factory | 工場棟・荷役ヤード |
| 補給所 | supply_depot | 倉庫・燃料保管・物資 |
| 軍港 | military_port | 岸壁・クレーン・港務施設 |

旧ファイル名 `mlrs`、`sam`、`jet`、`bomber_drone` も、それぞれ同じ種類のウクラヴィア側画像で更新します。旧塹壕画像 `trench_fort` は補給所・軍港には使用しません。

## 確認

`cargo test --release -p yydrone-game` で、兵種間・施設間の画像の重複、陣営別型式の区別、RGBA寸法・透過を確認します。画像キャッシュも陣営と兵種の組をキーに保持するため、一方の陣営を先に描画しても他陣営の画像へ影響しません。

## 外観の参考資料

- [F-16 / Lockheed Martin](https://www.lockheedmartin.com/en-us/products/f-16.html)
- [Bayraktar TB2 / Baykar](https://www.baykartech.com/en/uav/bayraktar-tb2/)
- [Javelin / Raytheon](https://www.rtx.com/raytheon/what-we-do/land)
- [パンツィリ / Rostec](https://rostec.ru/media/news/kompleks-pantsir-nebo-na-zamke/)
- [BMP-3・ムスタ・パンツィリ等の製品案内 / Rosoboronexport](https://roe.ru/upload/pdf/18182_post.pdf)
- [レレカ / ウクライナ空中強襲軍](https://dshv.mil.gov.ua/page/ukrayinska-leleka-ne-daye-spokoyu-okupantam-na-nashij-svyatij-zemli)
- [レレカ / 製造元DeViRo](https://deviro.ua/produkciya/leleka/)
- [MAGURA V5 / ウクライナ国防省情報総局](https://gur.gov.ua/content/magura-v5-zbroia-iaka-harantuie-morske-pravo-ukrainy.html)
