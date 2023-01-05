# チュートリアル

## インストール

```
# cargo install senax
```

## 初期ファイル

初期ディレクトリ構成の参考のため、 https://github.com/yossyX/senax から examples/actix のコードを展開してください。
とりあえずWebフレームワークとしては actix-web を使用しています。
tonic や hyper 等でも対応できるようにしたいと思いますが、動作は未確認です。
非同期ランタイムが tokio をそのまま利用しているとasyncブロックが Send である必要があり、その場合 SenaX は動作しません。
actix-web は tokio 上で動作していますが、非同期については !Send で動作するスレッドごとの動作になっています。
また、内部での消費メモリの計算はメモリアロケータとして mimalloc を使用することを想定しています。

## envファイル
```
$ cp .env.sample .env
```
.envのSAMPLE_DB_URLなどのDB設定を必要に応じて変更してください。

## スキーマファイル
スキーマファイルは基本的には設定ファイルとグループごとのモデル記述ファイルに分けて記述します。
シンプルなケースでは設定ファイル内にまとめて記述することもできます。

schema/sample.yml
```yml
# yaml-language-server: $schema=https://github.com/yossyX/senax/releases/download/0.1.3/schema.json#definitions/ConfigDef

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
yaml-language-serverの設定はVSCodeであればYAMLのプラグインを使用することにより、入力時にアシスタントが効きます。  

トップのsampleはDBのデータベース名です。  
dbはまだMySQLしか対応していません。  
tx_isolationとread_tx_isolationは更新用と参照用のデータベースの接続を分けて、それぞれread_committedとrepeatable_readにしています。  
MySQLはデフォルトでrepeatable_readですが、負荷がかかると結構ギャップロックにやられます。PostgreSQLはread_committedですし、多くの場合read_committedのほうが適切です。ただし、SELECT FOR UPDATEでヒットしなかったときにINSERTの手順はrepeatable_readでないとロックが効きませんが、これこそがギャップロックで死ぬパターンです。この場合、上位テーブルでロックを掛けるか、INSERT IGNOREや、ON DUPLICATE KEY UPDATEなどのupsert構文で対応が必要です。  
参照時のトランザクションはMySQLでは最初にテーブルにアクセスしたときにすべてのテーブルのスナップショットを取得するのでrepeatable_readが安全です。  
groupsは一つのデータベースに数百のテーブルがあるとディレクトリツリーが長くなったり管理が困ることになりますので、1階層分グループ分けできるようになっています。

schema/sample/note.yml
```yml
# yaml-language-server: $schema=https://github.com/yossyX/senax/releases/download/0.1.3/schema.json#properties/model

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

tag:
  columns:
    id:
      type: int
      primary: true
      auto_increment: auto
    note_id: int_not_null
    name: varchar_not_null
  relations:
    note:

category:
  columns:
    id:
      type: int
      primary: true
      auto_increment: auto
    name: varchar_not_null

counter:
  counting: counter
  timestampable: none
  use_save_delayed: true
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
Symfony 1.x の doctrine/schema.yml に近いイメージです。  
テーブル名がマルチバイト文字や複数形にも対応しています。
note モデルは tag と紐づいていますが、 relations の type が many では複数形の tags で自動的に紐づきます。  

ちなみに、 Doctrine の inheritance も一通り対応しています。  


セッション用の設定ファイルをダウンロードします。
```
$ wget -O schema/session.yml https://github.com/yossyX/senax/releases/latest/download/session.yml
```

## モデル生成
senaxのコマンドでdb/sample下にクレートを生成します。  
```
$ senax model sample
$ senax model session
```
note.rs等はカスタマイズ用で、_note.rsが本体です。再度モデル生成を実行するとnote.rsは上書きされず、_note.rsは常に上書きされます。  
モデル名が_Tagsのように_が付くのは他のライブラリと被らないようにそのような命名規則になっています。  
Dockerでの開発環境でVS Code等を使用していてモデルの再生成後に変更が反映されていない場合は、ウィンドウの再読み込みや生成されたモデルのファイルをVS Code上で再度保存するなどしてモデルの更新をVS Codeに認識させる必要があります。

## マイグレーション生成
db/sample/migrations下にDDLファイルを生成します。  
```
$ senax gen-migrate sample init
$ senax gen-migrate session init
```
DDLは現状のDBを確認して差分を出力します。  
DB仕様書の更新履歴出力のためにコメント部分に更新内容が出力されています。コメントの追加や不要な更新内容を削除して仕様書に出力される更新内容を変更することができます。

※ テーブル名はデフォルトでグループ名とモデル名を結合した名前になります。変更する場合はスキーマで table_name を指定してください。
また、 plural_table_name の設定でテーブル名を複数形で生成することが出来ます。

## マイグレーション実行
```
$ cargo run -p db_sample -- migrate -c
$ cargo run -p db_session -- migrate -c
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
db/sample/seeds/20221120120831_init.yml (数値部分は生成日時によって変化します。)
```yml
# yaml-language-server: $schema=../seed-schema.json

note:
  category:
    diary:
      name: diary
  note:
    note_1:
      category_id: diary
      key: sample
      content: content
  tag:
    tag_1:
      note_id: note_1
      name: tag 1
```
category_id の「diary」と note_id の「note_1」はそれぞれオートインクリメントで登録されたIDが渡されるようになっています。

## シードデータ投入
```
$ cargo run -p db_sample -- seed
```

## DBテーブル定義書生成

DB仕様書のER図出力のために graphviz が必要となります。
```
# apt-get install graphviz
```

```
$ senax gen-db-doc sample -e -H 10 > db-document.html
```
環境変数のLC_ALL, LC_TIME, LANGの設定により日本語の定義書を生成します。
"-e"はER図出力、"-H 10"は仕様書更新履歴を10件分出力します。

## コード記述

_Noteを取得して日毎のカウンターを加算しています。
save_delayedではこの処理が終わった後で同一の更新対象をまとめてaddの内容を加算して更新します。その更新内容をキャッシュに反映して他のサーバにも伝達します。

server/src/routes/api/cache.rs
```rust
#[get("/cache/{key}")]
async fn handler(
    key: web::Path<String>,
    http_req: HttpRequest,
    session: Session<_SessionStore>,
) -> impl Responder {
    let ctx = get_ctx_and_log(&http_req);
    let result = async move {
        // セッションの更新例
        let session_count = session
            .update(|s| {
                let v: Option<u64> = s.get_from_base(SESSION_KEY)?;
                match v {
                    Some(mut v) => {
                        v += 1;
                        s.insert_to_base(SESSION_KEY, v)?;
                        Ok(v)
                    }
                    None => {
                        s.insert_to_base(SESSION_KEY, 1)?;
                        Ok(1)
                    }
                }
            })
            .await?;

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
        let counter = _Counter::find_optional_from_cache(&conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1; // ここまではキャッシュから当日のカウント取得
        let mut counter_for_update = _CounterFactory { // 当日のカウントが未登録の場合 INSERT
            note_id: note.id(),
            date,
            counter: 0,
        }
        .create(&conn);
        let _ = counter_for_update.counter().add(1); // UPDATE加算
        counter_for_update._upsert(); // INSERT ... ON DUPLICATE KEY UPDATE の指示
        _Counter::update_delayed(&mut conn, counter_for_update).await?;

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
async fn handler(
    key: web::Path<String>,
    http_req: HttpRequest,
    session: Session<_SessionStore>,
) -> impl Responder {
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
        let counter = _Counter::find_optional(&mut conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1;

        let note_id = note.id();
        let cond = db_sample::cond_note_counter!((note_id=note_id) AND (date=date)); // WHERE句を生成するマクロ
        conn.begin().await?;
        let mut update = _Counter::for_update(&mut conn); // 更新内容を指定するための空のUpdate用オブジェクト生成
        let _ = update.counter().add(1);
        _Counter::query()
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
この例ではWHERE句の使用例のため、UPDATEクエリーで記述しています。
また、簡略化のため当日のカウンターが未登録の場合を考慮していません。  
upsert を使用すれば未登録の場合は登録、すでに登録されている場合は更新を実行できます。  

## サーバ起動
```
$ cargo run -p server
```
http://localhost:8080/api/cache/sample にアクセスして結果を確認できます。

本番環境ではリリースモードでビルドして起動します。
```
$ cargo build -p server -r
$ target/release/server
```

チュートリアル用のサンプルですが、ホットデプロイにも対応しています。
