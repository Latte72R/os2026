# vertos

![](./vertos.png)

Rustで実装された，RV64IMAC 向けのシンプルなOSです．

OpenSBI上のS-modeカーネルと，U-modeで動く対話シェルを実装しています．
ページングを使わないBareアドレス空間で，ECALL，独立したkernel/user stack，
協調式round-robin，spawn，yield，exit，waitに加え，foreground jobの停止・再開を
操作できます．

## 必要なツール

- [Rust](https://www.rust-lang.org/tools/install)
- [QEMU](https://www.qemu.org/)（`qemu-system-riscv64`）

## 使い方

カーネルのビルドとQEMUでの実行には，それぞれ次のコマンドを使用します．

```sh
make build  # カーネルをビルド
make run    # ビルドしたカーネルをQEMUで実行
```

そのほか，ソースコードの整形や検査もMakefileから実行できます．

```sh
make fmt    # Rustソースを整形
make check  # カーネルをビルドせずに検査
make test   # QEMUでカーネルのテストを実行
make test-shell # シェルとプロセス機構のE2Eテスト
make clean  # ビルド生成物を削除
```

シェルでは次のコマンドを利用できます．

```text
help echo ps jobs workers
yes fg bg kill wait clear poweroff
```

`workers`は2つのU-mode workerをforegroundで各10ステップ実行し，round-robinに
よる交互の出力を表示します．
`yes`は停止操作を試すために終了せず動き続けます．
実行中に`Ctrl+Z`を入力すると停止し，`jobs`で状態を確認して`fg`で再開できます．
`Ctrl+C`はforeground jobを終了します．
これは協調式ジョブ制御なので，プロセスが`yield`またはsystem callを呼んだタイミングで操作が反映されます．

PIDを省略した`fg`と`bg`は直近のjobを対象にします．
`workers`のjobは2 workerをまとめて保持するため，両方を同時に再開します．PIDを省略した`wait`はシェルの
全子プロセスを待機して終了状態を回収します．

シェルの入力中に`Ctrl+C`を押すと現在の行を破棄して新しいプロンプトへ戻ります．
左右の矢印キーはカーソル移動，上下の矢印キーは直近8件のコマンド履歴を移動します．
カーソルの途中での文字挿入とBackspaceにも対応しています．

`yes`や`workers`のようにU-modeプロセスとして起動できるコマンドをbackground実行します．
`ps`などのシェル内蔵コマンドは独立したプロセスではないため，`&`を付けるとエラーになります．

QEMUを終了するには，`Ctrl-a`を押してから`x`を押します．

## ディレクトリ構成

```text
boot/       起動処理と例外エントリのアセンブリ
linker/     カーネル用リンカスクリプト
scripts/    QEMU起動スクリプト
src/        Rustで実装したカーネル本体
```

## ライセンス

このプロジェクトは[MIT License](./LICENSE)のもとで公開されています．
