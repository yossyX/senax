# チュートリアル

## インストール

```
cargo install senax
```

graphviz と protobuf-compiler が必要となります。
```
apt-get install graphviz protobuf-compiler
```

## 初期ファイル

初期ディレクトリ構成の参考のため、 https://github.com/yossyX/senax から examples/actix のコードを展開してください。
とりあえず actix-web を使用していますが、tonic や hyper 等でも対応できるはずです。
また、消費メモリの計算は mimalloc を使用することを想定しています。

## スキーマファイル
schema/conf.yml
```yml
# yaml-language-server: $schema=https://github.com/yossyX/senax/releases/download/0.1.2/schema.json#properties/conf

sample:
  title: Sample DB
  author: author name
  db: mysql
  ignore_foreign_key: true
  timestampable: real_time
  time_zone: local
  timestamp_time_zone: utc
  tx_isolation: read_committed
  read_tx_isolation: repeatable_read
  use_cache: true
  use_fast_cache: true
  use_cache_all: true
  preserve_column_order: true
  groups:
    note:
      title: note
```
複数のデータベース設定のための設定ファイルです。  
yaml-language-serverの設定はVSCodeであればYAMLのプラグインで入力時にアシスタントが効きます。  

トップのsampleはDBのデータベース名です。  
dbはまだMySQLしか対応していません。  
tx_isolationとread_tx_isolationは更新用と参照用のデータベースの接続を分けて、それぞれread_committedとrepeatable_readにしています。  
MySQLはデフォルトでrepeatable_readですが、負荷がかかると結構ギャップロックにやられます。PostgreSQLはread_committedですし、多くの場合read_committedのほうが適切です。ただし、SELECT FOR UPDATEでヒットしなかったときにINSERTの手順はrepeatable_readでないとロックが効きませんが、これこそがギャップロックで死ぬパターンです。この場合、上位テーブルでロックを掛けるか、INSERT IGNOREや、ON DUPLICATE KEY UPDATEなどのupsert構文で対応が必要です。  
参照時のトランザクションはMySQLでは最初にテーブルにアクセスしたときにすべてのテーブルのスナップショットを取得するのでrepeatable_readが安全です。  
groupsは一つのデータベースに数百のテーブルがあるとディレクトリツリーが長くなったり管理が困ることになりますので、1階層分グループ分けできるようになっています。

schema/sample/note.yml
```yml
# yaml-language-server: $schema=https://github.com/yossyX/senax/releases/download/0.1.2/schema.json#properties/model

note:
  timestampable: fixed_time
  soft_delete: time
  versioned: true
  on_delete_fn: true
  use_fast_cache: true
  columns:
    id:
      type: int
      primary: true
      auto_increment: auto
    key: varchar
    category_id: int
    content:
      type: text
      length: 2000
      not_null: true
  relations:
    category:
      use_cache: true
    tags:
      type: many
      in_cache: true
  indexes:
    key:
      fields:
        key:
          length: 20
      type: unique
    content:
      type: fulltext
      parser: ngram

tags:
  columns:
    id:
      type: int
      primary: true
      auto_increment: auto
    note_id: int_not_null
    name: varchar_not_null
  relations:
    note:

categories:
  columns:
    id:
      type: int
      primary: true
      auto_increment: auto
    name: varchar_not_null

counters:
  counting: counter
  timestampable: none
  columns:
    note_id:
      type: int
      primary: true
    date:
      type: date
      primary: true
    counter: bigint_not_null
  relations:
    note:
```
グループ内のテーブルの設定ファイルです。  
Symfony 1.xのdoctrine/schema.ymlに近いイメージです。  
テーブル名がマルチバイト文字や複数形にも対応しています。categories テーブルはノートのrelationsのcategoryと紐づいていますが、relationsの方はtypeがデフォルトでoneなので単数形で自動的に紐づきます。  
ちなみに、 Doctrine の inheritance も一通り対応しています。  

## モデル生成
```
$ senax model sample
```
senaxのコマンドでdb/sample下にクレートを生成します。  
note.rs等はカスタマイズ用で、_note.rsが本体です。再度モデル生成を実行するとnote.rsは上書きされず、_note.rsは常に上書きされます。  
モデル名が_Tagsのように_が付くのは他のライブラリと被らないようにそのような命名規則にしています。  

## マイグレーション生成
```
$ senax gen-migrate sample init
```
db/sample/migrations下にDDLファイルを生成します。  
DDLは現状のDBを確認して差分を出力します。  
コマンド実行前に .env の SAMPLE_DB_URL で指定されたデータベースを作成しておいてください。  

※ テーブル名はデフォルトでグループ名とモデル名を結合した名前になります。変更する場合はスキーマで table_name を指定してください。

## マイグレーション実行
```
$ cargo run -p db_sample -- migrate -c
```

sqlxのマイグレーションを実行します。
シャーディング設定がある場合すべてのシャードにクエリーを発行するようになっています。  
-c はクリーンマイグレーションで、DBを作成してからマイグレーションを実行します。

## シードスキーマ生成
```
$ cargo run -p db_sample -- gen-seed-schema > db/sample/seed-schema.json
```
もしくは、
```
$ target/debug/db_sample gen-seed-schema > db/sample/seed-schema.json
```

シードファイルの入力アシスタントのためのスキーマファイルを生成します。

## シードファイル生成
```
$ senax gen-seed sample init
```
シードファイルもマイグレーションと同様にDBに登録済みかのバージョン管理を行いますので、コマンドでシードファイルを生成します。

## シードファイル記述
db/sample/seeds/20221120120831_init.yml
```yml
# yaml-language-server: $schema=../seed-schema.json

note:
  categories:
    diary:
      name: diary
  note:
    note_1:
      category_id: diary
      key: sample
      content: content
  tags:
    tag_1:
      note_id: note_1
      name: tag 1
```
「diary」と「note_1」はそれぞれオートインクリメントで登録されたIDが渡されるようになっています。

## シードデータ投入
```
$ cargo run -p db_sample -- seed
```

## テーブル定義書生成
```
$ senax gen-db-doc sample -e > db-document.html
```
環境変数のLC_ALL, LC_TIME, LANGの設定により日本語の定義書を生成します。

## コード記述

_Noteを取得して日毎のカウンターを加算しています。
save_delayedではこの処理が終わった後で同一の更新対象をまとめてaddの内容を加算して更新します。その更新内容をキャッシュに反映して他のサーバにも伝達します。

server/src/routes/api/cache.rs
```rust
#[get("/cache/{key}")]
async fn handler(key: web::Path<String>, http_req: HttpRequest) -> impl Responder {
    let ctx = get_ctx_and_log(&http_req);
    let result = async move {
        let mut conn = SampleConn::new();
        let mut note = _Note::find_by_key_from_cache(&conn, &*key) // ユニークキーからのキャシュ取得
            .await
            .with_context(|| NotFound::new(&http_req))?;
        note.fetch_category(&mut conn).await?; // これもキャシュから取得

        let category = match note.category() {
            None => None,
            Some(v) => Some(v.name().to_owned()),
        };

        let date = Local::now().date_naive();
        let counter = _Counters::find_optional_from_cache(&conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1; // ここまではキャッシュから当日のカウント取得
        let mut counter_for_update = _CountersFactory { // 当日のカウントが未登録の場合 INSERT
            note_id: note.id(),
            date,
            counter: 0,
        }
        .create(&conn);
        let _ = counter_for_update.counter().add(1); // UPDATE加算
        counter_for_update._upsert(); // INSERT ... ON DUPLICATE KEY UPDATE の指示
        _Counters::save_delayed(&mut conn, counter_for_update).await?;

        Ok(Response {
            id: note.id(),
            category,
            article: note.content().to_string(),
            tags: note.tags().iter().map(|v| v.name().to_string()).collect(),
            count,
        })
    }
    .await;
    json_response(result, &ctx)
}
```

キャッシュを使用しないバージョンです。

server/src/routes/api/no_cache.rs
```rust
#[get("/no_cache/{key}")]
async fn handler(key: web::Path<String>, http_req: HttpRequest) -> impl Responder {
    let ctx = get_ctx_and_log(&http_req);
    let result = async move {
        let mut conn = SampleConn::new();
        let mut note = _Note::find_by_key(&mut conn, &*key)
            .await
            .with_context(|| NotFound::new(&http_req))?;
        note.fetch_category(&mut conn).await?;
        note.fetch_tags(&mut conn).await?;

        let category = match note.category() {
            None => None,
            Some(v) => Some(v.name().to_owned()),
        };
        let date = Local::now().date_naive();
        let counter = _Counters::find_optional(&mut conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1;

        let note_id = note.id();
        let cond = db_sample::cond_note_counters!((note_id=note_id) AND (date=date));
        conn.begin().await?;
        let mut update = _Counters::for_update(&mut conn);
        let _ = update.counter().add(1);
        _Counters::query()
            .cond(cond)
            .update(&mut conn, update)
            .await?;
        conn.commit().await?;
        
        Ok(Response {
            id: note.id(),
            category,
            article: note.content().to_string(),
            tags: note.tags().iter().map(|v| v.name().to_string()).collect(),
            count,
        })
    }
    .await;
    json_response(result, &ctx)
}
```