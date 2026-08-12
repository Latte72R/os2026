# beigecore 統合向け OS 設計

この文書は、vertos を OpenSBI 上の S-mode カーネルとして動かし、beigecore の
Verilator シミュレータから操作できる状態までの OS 設計を定める。

CPU、SoC、OpenSBI、メモリマップの設計は beigecore リポジトリの
`docs/vertos-integration.md` を正本とする。

## 方針

現在実装済みの例外処理、First Fit Allocator、固定長プロセステーブル、
コンテキストスイッチを育てる。Sv39 page table は最初の完成点から外し、
`satp.MODE=Bare` の no-MMU OS とする。別の大規模フレームワークへの置き換えは
行わない。

最初の完成点では、ファイルシステムや動的 ELF loader を作らず、ビルド時に組み
込んだ小さなユーザープログラムを U-mode で実行する。スケジューラは既存実装を
活かした協調式 round-robin とし、プロセスが syscall `yield` を呼んだ時に切り
替える。

この構成でも、次を実際に確認できる。

- U-mode/S-mode/M-mode 間の遷移
- syscall と trap frame
- U-mode と S-mode の syscall/trap 境界
- kernel stack と user stack
- context switch
- spawn、exit、wait と PID
- UART を使う対話 CUI

## カーネルとプロセス

プロセステーブルは引き続き固定長 8 slot とし、動的コンテナにはしない。PID 0 は
idle/kernel context、PID 1 は最初の shell とする。

状態は最低限、次を持つ。

- `Idle`
- `Runnable`
- `Exited(code)`

端末入力は non-blocking syscall と `yield` の組み合わせで待つため、最初は複雑な
wait queue や `Blocked` 状態を追加しない。必要性が生じた時点で追加する。

通常プロセスは次を所有する。

- PID、parent PID、state
- 保存済み kernel context
- kernel stack
- user register/trap context
- user stack
- 組み込み program ID と初期引数

trap entry は既存の `sscratch` を用いる方式を維持する。U-mode 実行中は
`sscratch=kernel_stack_top`、S-mode 実行中は `sscratch=0` とする。プロセス切替時
には次プロセスの kernel stack と user context をまとめて切り替える。

## Bare 物理アドレス空間

カーネルは `0x8020_0000` にリンクする。8 MiB の物理 RAM に収めるため、初期
ヒープは 4 MiB 以下にし、linker script に RAM 終端を越えない ASSERT を置く。

`satp` はゼロのままとし、カーネルと全ユーザープロセスは同じ物理アドレス空間を
共有する。ユーザーコードは実 RAM 内の固定物理アドレスへリンクして一度だけ配置し、
各プロセスには独立した kernel stack、user stack、register context を割り当てる。

| 物理範囲 | 用途 |
| --- | --- |
| `0x8000_0000...` | OpenSBI firmware |
| `0x8020_0000...` | vertos kernel、kernel stack、heap |
| RAM 上部の予約領域 | 組み込み user image と user stack |

MMU/PMP がないため、U-mode から kernel memory、他プロセスの memory、MMIO への
アクセスをハードウェアでは禁止できない。これは最初の完成点の明示的な制約とし、
「保護されたプロセス」ではなく「独立 context を持つ U-mode task」と呼ぶ。

最初のユーザープログラムは raw binary としてカーネルへ埋め込み、固定物理
アドレスへ copy する。ELF parser と filesystem は導入しない。program registry
は固定の enum と byte slice の表で表現する。

## Syscall ABI

U-mode からの `ecall` を S-mode trap handler で処理する。番号は `a7`、引数は
`a0..a5`、結果は `a0` と必要に応じて `a1` で返す。戻る前に `sepc` を 4 byte
進める。

最初に実装する syscall は次のとおり。

| 番号 | 名前 | 動作 |
| --- | --- | --- |
| 0 | `putchar(ch)` | 1 byte を console へ出す |
| 1 | `getchar()` | 入力 1 byte、未入力なら `-1` |
| 2 | `yield()` | 次の Runnable process へ切り替える |
| 3 | `exit(code)` | 現プロセスを Exited にする |
| 4 | `spawn(program, arg)` | 組み込み program を生成し PID を返す |
| 5 | `wait(pid)` | non-blocking で Running/Exited と終了値を返す |
| 6 | `proc_info(slot)` | PID、親、状態をレジスタ値で返す |
| 7 | `shutdown()` | SBI System Reset を呼ぶ |

最初は文字列 syscall や共有メモリ ABI を作らず、ユーザーライブラリ側で
`putchar` を繰り返す。これにより user pointer 検証を console 完成の前提にしない。

## SBI と platform 層

QEMU と beigecore のカーネル本体を分岐させず、差は `platform` module 内へ閉じ
込める。

- 出力: legacy console putchar
- 入力: legacy console getchar
- 終了: SBI System Reset extension
- 将来の timer: SBI TIME extension

OpenSBI v1.9 は legacy console extension を既定で有効にしているため、最初は現在の
出力実装を拡張して入力を追加する。SBI call の戻り規約は legacy console と
v0.2 以降の extension で異なるため、共通関数で無理に同一視せず型の分かる wrapper
を用意する。

## ユーザーシェル

シェルは U-mode の PID 1 として動かす。固定長 128 byte の行バッファを持ち、
ASCII printable、Enter、Backspace を扱う。heap、履歴、補完、引用規則は最初は
持たない。

コマンドは次に限定する。

- `help`: コマンド一覧
- `echo <text>`: 引数を表示
- `ps`: プロセステーブルを表示
- `run demo`: yield しながら表示する複数 worker を生成
- `wait <pid>`: 指定 PID の終了を待つ間も yield する
- `clear`: ANSI escape で画面を消す
- `shutdown`: SBI 経由で終了

未知のコマンドと引数エラーは短いメッセージを返し、panic しない。

`demo` の worker は有限回だけ行を表示し、各回で `yield` して終了する。これを
シェルと並行実行することで context switch、PID、exit/wait を端末上で確認する。

## ビルドとテスト

vertos の Makefile に次の責務を持たせる。

- kernel ELF と raw binary の生成
- shell/worker user binary の生成
- QEMU run/test
- beigecore 統合側から参照できる安定した出力パス

QEMU テストは現在の custom test framework を維持する。単体テストに加え、次の
カーネルレベルテストを追加する。

- U-mode entry と syscall return
- user ECALL と illegal instruction の区別
- 複数 task の独立した register context と stack
- spawn/yield/exit/wait の状態遷移
- 不正 syscall 番号と不正 program ID のエラー処理

beigecore 側の E2E では入力を pipe し、少なくとも boot banner、shell prompt、
`ps`、demo の交互出力、終了コードを検査する。

## 実装順序

1. SBI getchar と console abstraction を追加する。
2. trap handler で U-mode ECALL を処理できるようにする。
3. Process に user context、user stack、kernel stack top を追加する。
4. raw user image loader と U-mode entry を実装する。
5. syscall ABI と user library を実装する。
6. shell と worker を実装し、QEMU 上で完成させる。
7. beigecore/OpenSBI E2E へ接続する。
8. E2E 安定後、必要に応じて timer preemption を追加する。

## 最初の完成点に含めないもの

- filesystem と block device
- ELF parser、動的 `exec`、`fork`
- pipe、signal、shell job control
- network
- 複数 HART と SMP synchronization
- timer preemption
- Sv39、PTW、プロセス間メモリ保護
- GUI

これらは対話シェルとユーザープロセスが自作 CPU 上で安定動作した後に、学習効果と
デモ上の価値が大きいものから追加する。

## 参照資料

- [Operating System in 1,000 Lines](https://operating-system-in-1000-lines.vercel.app/ja/)
- [operating-system-in-1000-lines](https://github.com/nuta/operating-system-in-1000-lines)
- [Wasabi OS source at the referenced revision](https://github.com/hikalium/wasabi/tree/477ff16256c91c51bd38b7b787d232bd6119b766/os/src)
- [RISC-V SBI specification](https://github.com/riscv-non-isa/riscv-sbi-doc)
