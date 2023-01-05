# リレーション
取得時にJOINでのリレーション取得は行わず、クエリー実行後に対象の主キーをまとめて、リレーション先からIN句を使用したクエリーで取得します。  

## リレーションの取得
取得したオブジェクトや更新用オブジェクト、あるいは Vec<\_{モデル名}> などに fetch\_{リレーション名} のメソッドを使用すると、そのオブジェクトやリストに必要なリレーションを一回のクエリで取得してそれぞれ元のオブジェクトに保存します。

```rust
let mut list = _{モデル名}::query().select(&mut conn).await?;
list.fetch_{リレーション名}(&mut conn).await?;
```

## 3世代目の取得
リレーション先のリレーションのような3世代目の取得については Vec<&mut \_{モデル名}> にまとめれば fetch\_{リレーション名} のメソッドが使用可能です。  
その際、2世代目のリレーション先のトレイトが必要になります。

```rust
use {2世代目のリレーションmod}::*;

let mut list = _{1世代目モデル名}::query().select(&mut conn).await?;
list.fetch_{2世代目リレーション名}(&mut conn).await?;
let mut v: Vec<_> = list.iter_mut().map(|v| v.{2世代目リレーション名}()).flatten().collect();
v.fetch_{3世代目リレーション名}(&mut conn).await?;
```
