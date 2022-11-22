# ロガー

senax-loggerは下記の特徴があります。

* LTSV形式で出力します。
* 1秒毎にzstdで圧縮して出力します。ただし、圧縮率は低くなるため、ローテーション後に再圧縮が必要です。
* 1日毎にローテーションします。
* Linuxの場合、io_uringを使用します。
* request, responseなどLOG_FILEで指定されたログは別ファイルに出力できます。
* errorとwarnのログ通知を受けて処理をカスタマイズすることができます。

コード例
```rust
let (error_rx, warn_rx) = senax_logger::init(Some(time::macros::offset!(+9)))?;
```

error_rx, warn_rxを使用しない場合は、必ず戻り値を破棄してください。
```rust
senax_logger::init(Some(time::macros::offset!(+9)))?;
```
