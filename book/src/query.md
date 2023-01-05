# WHRER句マクロ

QueryBuilderで使用するWHRER句はマクロで記述されます。  
基本的に比較演算子の左側にカラム名、右側に値となります。  
マクロはそのままテキスト化ではなく、プリペアードステートメントに変換されますので、SQLインジェクションは発生しません。
他のテーブルとの結合はJOINではなく、EXISTSで判定します。膨大なブログから表示可能な新着数件を抽出などではJOINよりもEXISTSのほうが早く取得できます。

```rust
let cond = db_{DB名}::cond_{グループ名}_{モデル名}!(クエリー);
let cond = db_sample::cond_note_note!(color = _Color::c);
let cond = db_sample::cond_note_note!(content = "ddd");
let cond = db_sample::cond_note_note!(NOT(note_id = id));
let cond = db_sample::cond_note_note!(note_id BETWEEN (3, 5));
let cond = db_sample::cond_note_note!(note_id SEMI_OPEN (3, 5));
let cond = db_sample::cond_note_note!((note_id < c) AND (note_id < 20) AND (note_id IN (3,3)));
let cond = db_sample::cond_note_note!((NOT ((note_id < c) AND (note_id < 1) AND (note_id < 2))) OR (note_id < 3) OR (note_id < 4));
let cond = db_sample::cond_note_note!(note_id ANY_BITS c);
let cond = db_sample::cond_note_note!(ONLY_TRASHED AND (category EXISTS (name = "diary")));
let cond = cond.and(db_sample::cond_note_note!(json CONTAINS [3,5]));
let result = _Note::query().cond(cond).select(conn).await?;
```

SEMI_OPENはBETWEENに似ていますが、半開区間 3 <= note_id AND note_id < 5 のように変換されます。

order by については下記のようになります。
IS NULL ASCの指定も可能ですが、インデックスの最適化は得られない可能性があります。

```rust
let order = db_sample::order_by_note_note!(note_id IS NULL ASC, note_id ASC);
let result = _Note::query().order_by(order).select(conn).await?;
```
