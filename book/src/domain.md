# ドメイン

ORMのモデル層としてはカスタマイズ用の note.rs にロジックを実装したくなるところですが、自動生成されたクレートのビルドに非常に時間がかかるので、別途ドメイン用のクレートを用意してそちらにビジネスロジックを実装します。  
単純な方法としてはビジネスロジックのトレイトを定義して _Note にトレイトの実装を行うことで、 _Note にメソッドが追加されたように扱うことができます。  
クリーンアーキテクチャの場合はモデルとテーブルの乖離が大きいので、新しい構造体を定義してデータの詰替などを実装する必要があると考えられます。
その際に _Note の代わりに Note などの名前で新しい構造体を定義すれば、名前が混乱することもなくクリーンに実装できると思われます。
