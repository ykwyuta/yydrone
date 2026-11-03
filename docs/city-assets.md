# 都市画像の生成と組み込み

10都市の画像を組み込み image_gen ツールで個別生成。実在都市の建築を参考にしたゲーム用の縮景であり、実景写真や厳密な地理的復元ではありません。街並みは戦災前の健全な状態を意図しています。

原画像: `assets/map/city_sources/<city>.png`。ゲーム素材: `assets/map/city_<city>.png` と同名 `.rgba`（192×192、RGBA8、147456 bytes）。透過を維持して余白付きで縮小します。既存の都市名との対応は `crates/game/src/art.rs` の設定を維持。汎用 capital/port/industrial/town はそれぞれ kyiv/odesa/mariupol/bakhmut と共通です。

再変換: `python tools/prepare_city_assets.py`。以前の `create_modern_cities.py` は図形による旧素材を出力するため、新画像の更新には使用しません。

## 参考資料

- [キーウ観光案内](https://visit.kyiv.ua/en/discover-kyiv/landmarks)
- [デルジプロム / UNESCO](https://whc.unesco.org/en/tentativelists/6249/)
- [オデーサ歌劇場](https://operahouse.od.ua/en/about/)
- [リヴィウ・リノク広場](https://lviv.travel/en/places/attractions/ploshcha-rinok)
- [ドニプロの建築](https://gorod.dp.ua/archi/)
- [ロストフのドン川堤防](https://tourism.rostov-gorod.ru/top-ten-places-in-rostov/9251/)
- [マリウポリの都市景観](https://www.encyclopediaofukraine.com/display.asp?linkpath=pagesM%5CA%5CMariupol)
- [バフムトの戦前の街並み / AP](https://apnews.com/article/893a2f22a73c68162c6cb3bbd62efe91)
- [ドンバス・アリーナ / FC Shakhtar](https://shakhtar.com/en/club/infrastructure/donbass-arena/)
- [セヴァストポリの沈没船記念碑](https://maps.visicom.ua/i/POIFQOBW2B)

## 生成プロンプト

### kyiv

```text
Use case: stylized-concept. Asset type: transparent isometric city sprite for an existing modern strategy game. Create ONE coherent compact miniature city district inspired by Kyiv in its intact prewar architectural appearance, not a current documentary view. STYLE: professionally hand-painted realistic 2.5D strategy-game environment art, restrained natural colors, believable architecture, crisp readable silhouettes at 192px. Orthographic isometric view from above at 35 degrees, same scale and lighting for whole set, soft daylight from upper left. A small thin roughly diamond-shaped ground footprint with streets and sparse trees, not a tall plinth. Entire district isolated with genuinely transparent background and generous 8 percent margin on all sides. Centered full object, no crop, no horizon or sky, no backdrop, no cast shadow outside footprint. 5-8 carefully arranged buildings with one clear local landmark, coherent geography compressed artistically. No texts, labels, lettering, flags, borders, UI, watermarks, people, weapons, war damage, fantasy buildings, giant symbolic objects or floating parts. Square image. CITY DETAILS: Kyiv, Ukraine. Architectural focus: Saint Sophia Cathedral with white walls, green roofs and modest gold cupolas and its separate Ukrainian baroque bell tower; a small cluster of warm cream Khreshchatyk mid-rise blocks, one contemporary blue-gray office building, a leafy park. Cathedral is a real solid church with correctly seated domes, not floating symbols.
```

残り9都市の正確なプロンプトは `city-prompts.json` に保存。

