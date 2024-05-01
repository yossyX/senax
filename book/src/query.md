# WHRER句マクロ

QueryBuilderで使用するWHRER句はマクロで記述されます。  
基本的に比較演算子の左側にフィールド名、右側に値となります。  
マクロはそのままテキスト化ではなく、プリペアードステートメントに変換されますので、SQLインジェクションは発生しません。
他のテーブルとの結合はJOINではなく、EXISTSで判定します。膨大なブログから表示可能な新着数件を抽出などではJOINよりもEXISTSのほうが早く取得できます。

```rust
let filter = db_{DB名}::filter_{グループ名}_{モデル名}!(クエリー);
let filter = filter!(color = _Color::c);
let filter = filter!(content = "ddd");
let filter = filter!(NOT(note_id = id));
let filter = filter!(note_id BETWEEN (3, 5));
let filter = filter!(note_id RIGHT_OPEN (3, 5));
let filter = filter!((note_id < c) AND (note_id < 20) AND (note_id IN (3,3)));
let filter = filter!((NOT ((note_id < c) AND (note_id < 1) AND (note_id < 2))) OR (note_id < 3) OR (note_id < 4));
let filter = filter!(note_id ANY_BITS c);
let filter = filter!(ONLY_TRASHED AND (category EXISTS (name = "diary")));
let filter = filter.and(filter!(json CONTAINS [3,5]));
let result = _Note::query().filter(filter).select(conn).await?;
```

RIGHT_OPENはBETWEENに似ていますが、半開区間 3 <= note_id AND note_id < 5 のように変換されます。

order by については下記のようになります。
IS NULL ASCの指定も可能ですが、インデックスの最適化は得られない可能性があります。

```rust
let order = order!(note_id IS NULL ASC, note_id ASC);
let result = _Note::query().order_by(order).select(conn).await?;
```
