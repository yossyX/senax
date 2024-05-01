# GraphQL

# GraphQLについて

GraphQLはまだあまり一般的とはいえないかもしれません。
理由として、下記が挙げられます。
* 自分で一からサーバを実装するのは困難で特定のフレームワークを必要とする
* リレーションをたどって秘密情報が漏れそう
* アクセス制限が不安

メリットとしては複数のマスタの情報を一回で取得できますので、フォームの表示に必要な複数の項目などが効率良く取得できます。
Rust の async_graphql クレートでは先程あげた問題もなく比較的簡単に実装できます。

# GraphQLインタフェース生成

下記のコマンドで GraphQL のインタフェースを作成します。
```
$ senax graphql <server> <db> [group] [model]
```
server : 実装するサーバパッケージのPATH  
db : DB名  
group : グループ名（省略可）  
model : モデル名（省略可）

graphql.rs には下記の行がコメントアウトされているはずですので、コメントを解除して有効化します。
```rust
pub mod <db name>;
```
```rust
    async fn <db name>(&self) -> data::GqlQueryData {
        data::GqlQueryData
    }
```
```rust
    async fn <db name>(&self) -> data::GqlMutationData {
        data::GqlMutationData
    }
```

サーバ起動後、 ブラウザで http://localhost:8080/gql を表示すると GraphiQL の画面で動作確認が出来ます。

GraphQLの仕様としては[Relay](https://relay.dev/)には対応していません。[Urql](https://formidable.com/open-source/urql/)を推奨します。
また、クエリーの階層構造の一番上に取得、更新メソッドが来ることが一般的ですが、大量のメソッドが同列に並ぶのを防ぐため階層構造にしており、下記のようになります。

```
{
  data {
    note {
      note {
        find(id: 1) {
          id,
          content
        }
      }
    }
  }
}
```
階層構造のメリットとして階層ごとに権限の設定が可能ですので、管理が容易です。
権限設定の例のためダミーのログイン機能も実装されています。

識別子はデフォルトでスネークケースとなります。これは数字付きフィールド名などを変換したときの誤動作を防ぐためです。JavaScriptで一般的なキャメルケースにする場合は --camel-case を指定してください。  
rust も JavaScript もマルチバイト識別子に対応しているのにも関わらず、残念ながら GraphiQL はわざわざエラーを返してくれます。これが GraphQL の共通仕様かどうかはわかりませんが。

リレーション先の取得についてはスキーマで use_cache, in_cache が指定された1階層のみで、無限にリレーションをたどるような実装にはなっていません。
