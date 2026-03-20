---
name: tgltrk
description: >
  Toggl Track のタイムトラッキングを CLI から操作する。タイマーの開始・停止、
  タイムエントリの一覧・編集・削除・再開、プロジェクトとタグの管理が可能。
  ユーザーが Toggl Track、タイムトラッキング、タイマー、工数記録に言及した際に使用する。
---

## 前提条件

- API トークンが設定済みであること（`tgltrk auth login` または環境変数 `TOGGL_API_TOKEN`）

## クイックリファレンス

| 目的                       | コマンド                                              |
|----------------------------|-------------------------------------------------------|
| タイマー確認               | `tgltrk timer current`                                |
| タイマー開始               | `tgltrk timer start -d "説明" -p PROJECT_ID`          |
| タイマー停止               | `tgltrk timer stop`                                   |
| エントリ一覧               | `tgltrk entries list -n 10`                           |
| 日付フィルタ               | `tgltrk entries list --since 2024-01-01 --until 2024-01-31` |
| エントリ再開               | `tgltrk entries continue ENTRY_ID`                    |
| プロジェクト一覧           | `tgltrk projects list`                                |
| タグ一覧                   | `tgltrk tags list`                                    |

プログラム的に処理する場合は `--json` を付ける。単純な確認ならプレーンテキストの方がトークン効率が良い。

## グローバルオプション

- `--json` — JSON 形式で出力（メタデータ付きエンベロープ）
- `--workspace <ID>` — ワークスペース ID を指定（デフォルトを上書き）

## コマンド

### timer

```
tgltrk timer current
tgltrk timer start [-d "説明"] [-p PROJECT_ID] [--task TASK_ID] [-t tag1,tag2] [-b]
tgltrk timer stop
```

### entries

```
tgltrk entries list [--since YYYY-MM-DD] [--until YYYY-MM-DD] [-n COUNT]
tgltrk entries get ENTRY_ID
tgltrk entries edit ENTRY_ID [-d "説明"] [-p PROJECT_ID] [-t tag1,tag2] [-b true|false]
tgltrk entries delete ENTRY_ID
tgltrk entries continue ENTRY_ID
```

`continue` は元エントリと同じ設定（説明、プロジェクト、タグ等）で新しいタイマーを開始する。

### projects

```
tgltrk projects list
tgltrk projects get PROJECT_ID
tgltrk projects create "名前"
tgltrk projects update PROJECT_ID --name "新しい名前"
tgltrk projects delete PROJECT_ID
```

### tags

```
tgltrk tags list
tgltrk tags create "名前"
tgltrk tags update TAG_ID --name "新しい名前"
tgltrk tags delete TAG_ID
```

### cache

```
tgltrk cache status
tgltrk cache clear
```

## JSON 出力

```json
{
  "meta": { "cached": ["user"] },
  "data": { ... }
}
```

`meta.cached` はキャッシュから取得したエンティティ。空配列なら API から取得。削除操作では `data` は `null`。

## キャッシュ動作

- ユーザー情報・プロジェクト・タグを 72 時間キャッシュ（API レートリミット対策）
- プロジェクト/タグの変更操作で自動無効化
- `auth login` でアカウント切替時に全クリア
- 手動クリア: `tgltrk cache clear`

## エラー動作

エラーは stderr に `Error: ...` で出力、終了コード 1。認証未設定、API エラー、不正な日付形式などで発生。

## 制約

- ワークスペースの作成・変更は不可（選択のみ）
- レポート機能（概要・詳細・週次）は未対応
- 一括操作（バッチ削除等）は未対応
