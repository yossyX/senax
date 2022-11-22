# SenaXについて

SenaXはRustで書かれたORMで、キャッシュに特化しており非常に高速に動作します。
ORMにはよくクエリーキャッシュが実装されていますが、SenaXは一般的なクエリーキャッシュのような動作はせず、エンティティキャッシュを基本にしています。<br>
具体的な速度例として、DBからの取得とカウンター加算の更新を行うAPIのリクエストを32cpuで42万req/s程度の性能を出すことができます。

SenaXの特徴として次の点が挙げられます
* クエリー集約
* エンティティキャッシュ
* 遅延一括更新
* サーバ間キャッシュ更新同期

### クエリー集約
一般的なORMではAPIへのアクセスごとにそれぞれDBへクエリーを送り、その結果を返します。
SenaXではほぼ同時に行われたアクセスからのDBへのクエリーを一つにまとめ、一回のクエリーで結果を取得します。
具体的には主キーでの取得を IN クエリーに変換して取得します。

これにより、キャッシュにないデータにアクセスが殺到した場合のDBの負荷を低減します。

### エンティティキャッシュ
エンティティキャッシュはあらかじめ定義されたone-to-oneあるいはone-to-manyの下位の結合関係を持つテーブルのデータもまとめてキャッシュされます。
エンティティの取得は主キーだけではなく、ユニークキーからの取得も対応しています。

### 遅延一括更新
ログなどをDBに保存する際に、同時に受け付けた他のアクセスのログとまとめてバルクインサートで保存することができます。<br>
また、同一ページへのアクセスカウンタなどは同時にアクセスしたカウントアップ数をまとめてからDB更新も簡単にできます。

### サーバ間キャッシュ更新同期
キャッシュに保存されたデータが更新される場合、他のサーバにも更新内容を伝達してそれぞれのサーバのキャッシュが最新であるように同期されます。<br>
更新差分はカラムごとに処理されるため、別のサーバで同時に異なるカラムが更新されても問題ありません。
同じカラムが更新される場合はバージョン管理で競合を防ぐことができます。