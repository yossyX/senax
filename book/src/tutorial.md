# チュートリアル

## インストール

```
# cargo install senax
```

## 初期ファイル生成
```
$ senax init example
$ cd example
```

## DBテンプレート生成
```
$ senax init-db data
```
スキーマに data.yml を生成し、 data という名前のDBを使用することを設定します。

## envファイル
生成された .env ファイルの SESSION_DB_URL, DATA_DB_URL などのDB設定を必要に応じて変更してください。

## スキーマファイル
スキーマファイルは基本的には設定ファイルとグループごとのモデル記述ファイルに分けて記述します。
シンプルなケースでは設定ファイル内にまとめて記述することもできます。

下記にチュートリアル用の内容を記述していますので、該当するファイルを修正してください。

schema/data.yml
```yml
# yaml-language-server: $schema=../senax-schema.json#definitions/ConfigDef

title: Example DB
author: author name
db: mysql
ignore_foreign_key: true
timestampable: fixed_time
time_zone: local
timestamp_time_zone: utc
tx_isolation: read_committed
read_tx_isolation: repeatable_read
use_cache: true
use_all_row_cache: true
preserve_column_order: false
groups:
  note:
```
複数のデータベース設定のための設定ファイルです。  
yaml-language-serverの設定はVSCodeであればYAMLのプラグインを使用することにより、入力時にアシスタントが効きます。  
db は現在のところ MySQL しか対応していません。  

schema/data/note.yml に下記のファイルを作成してください。
```yml
# yaml-language-server: $schema=../../senax-schema.json#properties/model

note:
  soft_delete: time
  versioned: true
  fields:
    id:
      type: int
      primary: true
      auto: increment
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
  fields:
    id:
      type: int
      primary: true
      auto: increment
    note_id: int_not_null
    name: varchar_not_null
  relations:
    note:

category:
  fields:
    id:
      type: int
      primary: true
      auto: increment
    name: varchar_not_null

counter:
  counting: counter
  timestampable: none
  use_save_delayed: true
  fields:
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
note モデルは tag と紐づいていますが、 relations の type が many では複数形の tags で自動的に紐づきます。  

## モデル生成
senaxのコマンドで db/data, db/session 下にクレートを生成します。  
```
$ senax model data
$ senax model session
```
生成されたファイルの note.rs 等はカスタマイズ用で、 _note.rs が本体です。再度モデル生成を実行すると note.rs は上書きされず、 _note.rs は常に上書きされます。  
モデル名が _Tags のようにアンダースコアが付くのは他のライブラリと被らないようにそのような命名規則になっています。  
Docker での開発環境で VS Code 等を使用していてモデルの再生成後に変更が反映されていない場合は、ウィンドウの再読み込みや生成されたモデルのファイルを VS Code 上で再度保存するなどしてモデルの更新を VS Code に認識させる必要があります。

## マイグレーション生成
db/data/migrations下にDDLファイルを生成します。  
```
$ senax gen-migrate data init
$ senax gen-migrate session init
```
スキーマファイルを修正して再実行すると現状のDBを確認して差分のDDLを出力します。  
DB仕様書の更新履歴出力のためにコメント部分に更新内容が出力されています。コメントの追加や不要な更新内容を削除して仕様書に出力される更新内容を変更することができます。

※ テーブル名はデフォルトでグループ名とモデル名を結合した名前になります。変更する場合はスキーマで table_name を指定してください。
また、 plural_table_name の設定でテーブル名を複数形で生成することが出来ます。

## マイグレーション実行
```
$ cargo run -p db_data -- migrate -c
$ cargo run -p db_session -- migrate -c
```

sqlxのマイグレーションを実行します。
シャーディング設定がある場合すべてのシャードにクエリーを発行するようになっています。  
-c はクリーンマイグレーションで、DBを作成してからマイグレーションを実行します。

## シードスキーマ生成
```
$ cargo run -p db_data -- gen-seed-schema
```
もしくは、デフォルト動作するサーバ生成後は次のコマンドでも動作します。
```
$ cargo run -- gen-seed-schema data
```

シードファイルの入力アシスタントのためのスキーマファイルを生成します。

## シードファイル生成
```
$ senax gen-seed data init
```
シードファイルもマイグレーションと同様にDBに登録済みかのバージョン管理を行いますので、コマンドでシードファイルを生成します。

## シードファイル記述
db/data/seeds/20221120120831_init.yml (数値部分は生成日時によって変化します。)
```yml
# yaml-language-server: $schema=../seed-schema.json

note:
  category:
    diary:
      name: diary
  note:
    note_1:
      category_id: diary
      key: diary
      content: content
  tag:
    tag_1:
      note_id: note_1
      name: tag 1
```
category_id の「diary」と note_id の「note_1」はそれぞれ category と note が生成されたときのオートインクリメントで登録された ID が渡されるようになっています。

## シードデータ投入
```
$ cargo run -p db_data -- seed
```

## DBテーブル定義書生成

DB仕様書のER図出力のために graphviz が必要となります。
```
# apt-get install graphviz
```

```
$ senax gen-db-doc data -e -H 10 > db-document.html
```
環境変数のLC_ALL, LC_TIME, LANGの設定により日本語の定義書を生成します。
"-e"はER図出力、"-H 10"は仕様書更新履歴を10件分出力します。

## Actix サーバ生成
```
$ senax new-actix server --db data
```
serverの部分は任意のパッケージ名で、actix-web を使用したWebサーバを生成します。  
生成されるコードはSIGUSR2シグナルによるホットデプロイに対応しています。

## コード記述

TODO 下記のコードは example の一部なのでコードが不足しています。

_Noteを取得して日毎のカウンターを加算しています。
save_delayed ではこの処理が終わった後で同一の更新対象をまとめてaddの内容を加算して更新します。その更新内容をキャッシュに反映して他のサーバにも伝達します。

server/src/routes/api/cache.rs
```rust
use crate::context::Ctx;
use crate::response::*;
use actix_web::{get, web, HttpRequest, Responder};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::Local;
use db_sample::misc::Updater;
use db_sample::models::note::counter::*;
use db_sample::models::note::note::*;
#[allow(unused_imports)]
use db_sample::DbConn as SampleConn;
use db_session::models::session::session::{_SessionStore, senax_actix_session::Session};
use serde::Serialize;

const SESSION_KEY: &str = "count";

#[derive(Serialize)]
pub struct Response {
    pub id: _NoteId,
    pub category: Option<String>,
    pub article: String,
    pub tags: Vec<String>,
    pub count: u64,
    pub session_count: u64,
}

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

        let mut conn = DataConn::new();
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
        let mut counter_updater = _CounterFactory { // 当日のカウントが未登録の場合 INSERT
            note_id: note.id(),
            date,
            counter: 0,
        }
        .create(&conn);
        let _ = counter_updater.counter().add(1); // UPDATE加算
        counter_updater._upsert(); // INSERT ... ON DUPLICATE KEY UPDATE の指示
        _Counter::update_delayed(&mut conn, counter_updater).await?;

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
use crate::context::Ctx;
use crate::response::*;
use actix_web::{get, web, HttpRequest, Responder};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::Local;
use db_sample::models::note::counter::*;
use db_sample::models::note::note::*;
use db_sample::models::note::tag::_TagTr;
#[allow(unused_imports)]
use db_sample::DbConn as SampleConn;
use db_session::models::session::session::{_SessionStore, senax_actix_session::Session};
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub id: _NoteId,
    pub category: Option<String>,
    pub article: String,
    pub tags: Vec<String>,
    pub count: u64,
}

#[get("/no_cache/{key}")]
async fn handler(
    key: web::Path<String>,
    http_req: HttpRequest,
    session: Session<_SessionStore>,
) -> impl Responder {
    let ctx = get_ctx_and_log(&http_req);
    let result = async move {
        let mut conn = DataConn::new();
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
        let filter = db_data::filter_note_counter!((note_id=note_id) AND (date=date)); // WHERE句を生成するマクロ
        conn.begin().await?;
        let mut updater = _Counter::updater(&mut conn); // 更新内容を指定するための空のUpdate用オブジェクト生成
        let _ = updater.counter().add(1);
        _Counter::query()
            .filter(filter)
            .update(&mut conn, updater)
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
http://localhost:8080/api/cache/diary にアクセスして結果を確認できます。

本番環境ではリリースモードでビルドして起動します。
```
$ cargo build -p server -r
$ target/release/server
```
