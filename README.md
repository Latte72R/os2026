# vertos

![](./vertos.png)

Rustで実装された，RV64IMAC 向けのシンプルなOSです．

OpenSBI上のS-modeカーネルと，U-modeで動く対話シェルを実装しています．
ページングを使わないBareアドレス空間で，ECALL，独立したkernel/user stack，
協調式round-robin，spawn，yield，exit，waitを実際に操作できます．

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
help
echo hello
ps
run demo
wait 2
clear
shutdown
```

`run demo`は2つのU-mode workerを生成し，明示的な`yield`による切替を表示します．

QEMUを終了するには，`Ctrl-a`を押してから`x`を押します．

## ディレクトリ構成

```text
boot/       起動処理と例外エントリのアセンブリ
linker/     カーネル用リンカスクリプト
scripts/    QEMU起動スクリプト
src/        Rustで実装したカーネル本体
```

## beigecore との統合

U-mode プロセスと対話シェルを beigecore 上で動かすための構成と実装順序は
[beigecore 統合向け OS 設計](./docs/beigecore-integration.md)にまとめています．

## ライセンス

このプロジェクトは[MIT License](./LICENSE)のもとで公開されています．
