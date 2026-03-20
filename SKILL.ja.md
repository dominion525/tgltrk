---
name: tgltrk
description: >
  Toggl Track のタイムトラッキングを CLI から操作する。タイマーの開始・停止、
  タイムエントリの一覧・編集・削除・再開、プロジェクトとタグの管理が可能。
  ユーザーが Toggl Track、タイムトラッキング、タイマー、工数記録に言及した際に使用する。
---

## 前提条件

- API トークンが設定済みであること（`tgltrk auth login` または環境変数 `TOGGL_API_TOKEN`）

## ワークフロー

### タイマーを開始する

ユーザーが作業の記録を依頼したとき:

1. **情報が十分か判断する**
   - ユーザーがプロジェクト名・タグ・説明を具体的に指定している → そのまま `timer start` を実行
   - 曖昧な場合（「この作業を記録して」等）→ 次のステップへ

2. **プロジェクトとタグの候補を取得する**
   ```
   tgltrk projects list
   tgltrk tags list
   ```

3. **最適な候補を提案して確認する**
   - 作業内容から最も近いプロジェクトとタグを推測する
   - 「プロジェクト: X、タグ: Y で開始しますか？」とユーザーに確認する
   - 該当するものが見つからない場合は一覧を提示して選んでもらう

4. **確認が取れたら実行する**
   ```
   tgltrk timer start -d "説明" -p PROJECT_ID -t tag1,tag2
   ```

### タイマーを停止する

```
tgltrk timer stop
```

実行中タイマーがない場合はエラーになる。

### 現在のタイマーを確認する

```
tgltrk timer current
```

### 過去の作業を再開する

同じプロジェクト・タグ・説明で新しいタイマーを開始する:

```
tgltrk entries list -n 5
tgltrk entries continue ENTRY_ID
```

### エントリを修正する

指定したフィールドだけ更新される。全オプション任意。

```
tgltrk entries edit ENTRY_ID [-d "説明"] [-p PROJECT_ID] [-t tag1,tag2] [-b true|false]
```

## クイックリファレンス

| 目的                       | コマンド                                              |
|----------------------------|-------------------------------------------------------|
| タイマー確認               | `tgltrk timer current`                                |
| タイマー開始               | `tgltrk timer start -d "説明" -p PROJECT_ID`          |
| タイマー停止               | `tgltrk timer stop`                                   |
| エントリ一覧               | `tgltrk entries list -n 10`                           |
| 日付フィルタ               | `tgltrk entries list --since 2024-01-01 --until 2024-01-31` |
| エントリ再開               | `tgltrk entries continue ENTRY_ID`                    |
| エントリ編集               | `tgltrk entries edit ENTRY_ID -d "新しい説明"`        |
| エントリ削除               | `tgltrk entries delete ENTRY_ID`                      |
| プロジェクト一覧           | `tgltrk projects list`                                |
| プロジェクト作成           | `tgltrk projects create "名前"`                       |
| タグ一覧                   | `tgltrk tags list`                                    |
| タグ作成                   | `tgltrk tags create "名前"`                           |
| キャッシュ確認             | `tgltrk cache status`                                 |
| キャッシュクリア           | `tgltrk cache clear`                                  |

各コマンドの全オプションは `tgltrk <command> --help` で確認できる。

## グローバルオプション

- `--json` — JSON 形式で出力（メタデータ付きエンベロープ）。プログラム的に処理する場合に使用。単純な確認ならプレーンテキストの方がトークン効率が良い
- `--workspace <ID>` — ワークスペース ID を指定（デフォルトワークスペースを上書き）

## JSON 出力

```json
{
  "meta": { "cached": ["user"] },
  "data": { ... }
}
```

- `meta.cached` — キャッシュから取得したエンティティ（空配列なら API から取得）
- `data` — コマンドの結果。削除操作では `null`

## キャッシュ動作

- ユーザー情報・プロジェクト・タグを 72 時間キャッシュ（API レートリミット対策）
- プロジェクト/タグの作成・更新・削除で自動無効化
- `auth login` でアカウント切替時に全クリア
- 手動クリア: `tgltrk cache clear`

## エラー動作

エラーは stderr に `Error: ...` で出力、終了コード 1。認証未設定、API エラー、不正な日付形式などで発生。

## 制約

- ワークスペースの作成・変更は不可（`--workspace` は選択のみ）
- レポート機能（概要・詳細・週次）は未対応
- 一括操作（バッチ削除等）は未対応
