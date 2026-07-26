# azooKey for Windows

[AzooKeyKanaKanjiConverter](https://github.com/azooKey/AzooKeyKanaKanjiConverter)を利用したWindows版IMEです。

> [!WARNING]
> 現在開発中であるため、安定性や機能に関しては保証できません。使用する際は自己責任でお願いします。

# コミュニティ

## 開発を支援する
- [GitHub Sponsors (Miwa)](https://github.com/sponsors/ensan-hcl): 変換エンジンの開発者
- [Patreon (fkunn1326)](https://www.patreon.com/c/fkunn1326): Windowsに移植した人

## 開発に参加する

### 開発環境のセットアップ

- [Rust](https://www.rust-lang.org/tools/install)
- [Swift for Windows](https://www.swift.org/install/windows/) (Swift 6.0以上)
- [protoc](https://protobuf.dev/installation/) 
- [node.js](https://nodejs.org/en/download/)
- [inno setup](https://jrsoftware.org/isinfo.php)

### ビルド

#### リポジトリのクローン
```
git clone https://github.com/fkunn1326/azookey-Windows --recursive
```
`--recursive`オプションを付けて、サブモジュールも一緒にクローンしてください。

#### ビルド
```bash
cargo build -p tsf-core
```

デバッグのために特定のアプリだけにIMEを適用したいことがあります。そのときは、`IME_ALLOWED_APPS`環境変数を設定し、`app-restriction` featureを有効にしてください。

```bash
$Env:IME_ALLOWED_APPS="LINE.exe"; cargo build --features app-restriction
```

#### 開発時のヒント
- 開発は仮想マシンまたは専用のPCで行うことを推奨します。IMEがクラッシュするとWindowsがフリーズする可能性があります。
- IMEを解除する際、IMEを使用中のアプリケーション（メモ帳など）を終了しないと、解除できないことがあります。

# 関連

- [azooKey/azooKey](https://github.com/azooKey/azooKey): iOS / iPadOS向けの日本語キーボードアプリ
- [7ka-Hiira/fcitx5-hazkey](https://github.com/7ka-Hiira/fcitx5-hazkey): fcitx5向けのLinux版azooKey
- [azooKey/AzookeyKanakanjiConverter](https://github.com/azooKey/AzooKeyKanaKanjiConverter): azooKeyの変換エンジン

# 参考
本プロジェクトの開発にあたり、以下のリソースを参考にしています。
- [OMAMA-Taioan/khiin-rs](https://github.com/OMAMA-Taioan/khiin-rs/tree/master/windows)
- [google/mozc](https://github.com/google/mozc/tree/master/src/win32/tip)
- [microsoft/Windows-classic-samples](https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/Win7Samples/winui/input/tsf/textservice)
- [dec32/ajemi](https://github.com/dec32/ajemi)
- https://zenn.dev/mkpoli/scraps/6dc57fcd0335cf
