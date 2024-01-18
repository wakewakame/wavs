# 発生しうるエラー

js を以下のような手順で実行している

1. v8 の isolate を作成
2. js を実行する context を作成
3. context の console.log を取得する inspector を作成
4. context の中で js を実行する
5. js の中から呼ばれた console.log を inspector が受け取る
6. 2 に戻って同じことをしても、なぜか console.log の結果が受け取れない??

# 具体的なコード (簡易版)

```
isolate = new Isolate();

context1 = new Context();
inspector1 = new Inspector(isolate, context);
context1.exec("console.log('hello');");
```
