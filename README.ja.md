# tgltrk

[![CI](https://github.com/dominion525/tgltrk/actions/workflows/ci.yml/badge.svg)](https://github.com/dominion525/tgltrk/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)
[![MSRV: 1.85](https://img.shields.io/badge/MSRV-1.85-orange)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)

[English](README.md)

非公式の Toggl Track CLI ツール。Toggl Track API v9 の薄いラッパーとして、タイムトラッキングをコマンドラインから操作できます。

## インストール

### GitHub Releases

[Releases](https://github.com/dominion525/tgltrk/releases) からプラットフォームに合ったバイナリをダウンロードしてください。

対応プラットフォーム:
- Linux (x86_64)
- macOS (Apple Silicon / Intel)
- Windows (x86_64)

### ソースからビルド

```bash
cargo install --path .
```

Rust 1.85.0 以上が必要です。

## セットアップ

Toggl Track の API トークンを設定します。トークンは [Profile Settings](https://track.toggl.com/profile) から取得できます。

```bash
# 対話的に入力（推奨 - シェル履歴に残らない）
tgltrk auth login

# 環境変数で指定
export TOGGL_API_TOKEN=your_token_here
```

認証状態の確認:

```bash
$ tgltrk auth status
✓ Authenticated as Alice (alice@example.com)
```

## 使い方

### タイマー操作

```bash
# 現在のタイマーを確認
$ tgltrk timer current
#12345678 コードレビュー 01:23:45 [running]

# タイマーを開始
$ tgltrk timer start -d "コードレビュー" -p 12345 -t bug,review
✓ Timer started
#12345679 コードレビュー 00:00:00 [running] [bug, review]

# タイマーを停止
$ tgltrk timer stop
✓ Timer stopped
#12345679 コードレビュー 00:45:12
```

### タイムエントリ

```bash
# 最近のエントリを表示
$ tgltrk entries list -n 3
#12345677 週次ミーティング 01:00:00
#12345676 バグ修正 00:30:15
#12345675 デザインレビュー 02:15:00 [design]

# 日付範囲でフィルタ
$ tgltrk entries list --since 2024-01-01 --until 2024-01-31

# エントリの詳細
$ tgltrk entries get 12345677
#12345677 週次ミーティング 01:00:00

# 過去のエントリを作成（時刻指定）
$ tgltrk entries create --start "2024-01-15 09:00" --stop "2024-01-15 10:30" -d "朝の打ち合わせ"
✓ Entry created
#12345678 朝の打ち合わせ 01:30:00

# duration で作成（stop の代わり）
$ tgltrk entries create --start "2024-01-15 14:00" --duration "2h" -d "コーディング"
✓ Entry created
#12345679 コーディング 02:00:00

# エントリを編集
$ tgltrk entries edit 12345677 -d "朝会" -b true
✓ Entry updated
#12345677 朝会 01:00:00

# 開始・終了時刻を修正
$ tgltrk entries edit 12345677 --start "2024-01-15 09:30" --duration "45m"
✓ Entry updated
#12345677 朝会 00:45:00

# エントリを削除
$ tgltrk entries delete 12345677
✓ Entry #12345677 deleted

# 過去のエントリを再開（同じ設定で新しいタイマーを開始）
$ tgltrk entries continue 12345676
✓ Timer continued
#12345680 バグ修正 00:00:00 [running]
```

### プロジェクト管理

```bash
$ tgltrk projects list
#1001 ウェブサイトリニューアル [株式会社A]
#1002 モバイルアプリ [株式会社A]
#1003 API移行 [株式会社B] (archived)

# クライアントを指定してプロジェクト作成
$ tgltrk projects create "新プロジェクト" --client 101
✓ Project created
#1004 新プロジェクト

$ tgltrk projects update 1004 --name "名前変更"
✓ Project updated
#1004 名前変更

$ tgltrk projects delete 1004
✓ Project #1004 deleted
```

### クライアント管理

```bash
$ tgltrk clients list
#101 株式会社A
#102 株式会社B

$ tgltrk clients get 101
#101 株式会社A

$ tgltrk clients create "新規クライアント"
✓ Client created
#103 新規クライアント

$ tgltrk clients update 103 --name "名前変更"
✓ Client updated
#103 名前変更

$ tgltrk clients delete 103
✓ Client #103 deleted
```

### タグ管理

```bash
$ tgltrk tags list
#501 bug
#502 review
#503 design

$ tgltrk tags create "urgent"
✓ Tag created
#504 urgent

$ tgltrk tags delete 504
✓ Tag #504 deleted
```

### ワークスペース

```bash
$ tgltrk workspaces list
#1234 マイワークスペース
#5678 チームワークスペース

$ tgltrk workspaces get 1234
#1234 マイワークスペース
```

### キャッシュ

ユーザー情報・プロジェクト・クライアント・タグ・ワークスペースの一覧は API 呼び出しを削減するため 72 時間キャッシュされます。Toggl Track API にはレートリミットがあり、キャッシュによって通常利用時にリミットに達することを防ぎます。

```bash
$ tgltrk cache status
Cache dir: /Users/alice/Library/Caches/com.tgltrk.tgltrk
  user: 245 bytes, last updated 2024-01-15 10:30:00 UTC
  projects_1234: 1024 bytes, last updated 2024-01-15 10:30:01 UTC

$ tgltrk cache clear
✓ Cache cleared
```

プロジェクトやタグを作成・更新・削除するとキャッシュは自動で無効化されます。`auth login` でアカウントを切り替えると、すべてのキャッシュがクリアされます。

## グローバルオプション

```
--json         JSON 形式で出力
--workspace    ワークスペース ID を指定（デフォルトワークスペースを上書き）
```

JSON 出力にはキャッシュヒット情報がメタデータとして含まれます:

```json
{
  "meta": { "cached": ["user"] },
  "data": {
    "email": "alice@example.com",
    "fullname": "Alice",
    "default_workspace_id": 1234,
    "timezone": "Asia/Tokyo"
  }
}
```

## 認証情報の管理

API トークンは OS のネイティブキーリング（macOS Keychain / Windows Credential Manager / Linux Secret Service）に保存されます。キーリングが利用できない環境では、環境変数 `TOGGL_API_TOKEN` を使用してください。

## 制限事項

- **一括操作**: 複数エントリ・プロジェクトの一括編集・削除は未対応です
- **レポート機能**: Toggl Track のレポートエンドポイント（概要・詳細・週次）は未対応です
- **ワークスペース管理**: ワークスペースの一覧表示は可能ですが、作成・変更はできません。`--workspace` は既存のワークスペースの選択のみです
- **有料プラン機能**: 有料プラン限定の機能（タスク、プロジェクトテンプレート、時間見積もり、必須フィールドなど）は未対応です
- **レートリミット**: Toggl Track API にはレートリミットがあります。CLI はデータを 72 時間キャッシュして API 呼び出しを最小限に抑えています。レートリミットに達した場合はリセットを待ってから再試行してください

### 未対応の API エンドポイント

以下の Toggl Track API v9 エンドポイントは利用可能ですが、この CLI では未実装です:

- `PUT /me` — ユーザープロフィール更新（タイムゾーン、メール等）
- `GET /me/projects`, `GET /me/clients`, `GET /me/tags` — ワークスペース横断の一覧取得（`--workspace` での単一ワークスペース指定による一覧は対応済み）
- `GET /me/features` — アカウントの機能フラグ
- `GET /me/location` — IP ベースの位置情報
- `GET /me/web-timer` — Web タイマーの状態（`timer current` で代替可能）
- `PATCH /workspaces/{wid}/time_entries/{ids}` — タイムエントリの一括更新
- `PATCH /workspaces/{wid}/projects/{ids}` — プロジェクトの一括更新

## ライセンス

MIT OR Apache-2.0
