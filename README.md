# toy-blog
## 概要
[`KisaragiEffective`](https://github.com/KisaragiEffective/) が開発する実験用のブログ。

## How to run
```sh
cargo run -- \
  --http-host=127.0.0.1 \
  --http-port=8080
```

### 解説
* `--bearer-token`: 廃止。標準入力から改行終端で与えること。
* `--http-host`: HTTPサーバーのホスト。通常は`127.0.0.1`を指定して良い。
* `--http-port`: HTTPサーバーのポート番号。
* `--read-bearer-token-from-stdin`: 次のメジャーバージョンで廃止予定。このスイッチはもはや互換性のためだけに残されている。

### 動作させるにあたっての注意事項
* Cloudflareのトンネルを使っている場合、`--cloudflare`スイッチを付け足すこと。

## 永続化
全てのデータはJSONで永続化される。

データはカレントディレクトリ直下の`data/`ディレクトリ以下に保存される。

### ディレクトリ構造
* カレントディレクトリ
  * `data`
    * `articles.json`
    * `cors_setting.json`

### `articles.json`
記事のデータを格納する。
* `data`
  * (map)
    * key: 記事ID
    * value
      * `created_at: date`: 作成日時
      * `updated_at: date`: 更新日時
      * `content: string` : 記事の本文

実装上の注: `GET /article/{article_id}`の応答速度を向上させるためにmapを用いている。

### `cors_setting.json`
CORSアクセスが許可されるプロトコル付きのFQDNを記述する。
* (array)
  * `protocol_and_fqdn: string` - プロトコル付きのFQDN。例えば、`https://my-frontend.example.com`

## API
APIのエンドポイントのベースは`http://{YOUR_DOMAIN}/api`である。HTTPSには対応していない。

### `GET /list/article`
現在登録されている記事のIDを配列形式で全て返す。この際、順序が何らかの一貫した順序付けになっているとは限らない。

#### レスポンス
* `200`: 指定された記事が (**0件以上**) 見つかった。`Content-Type`は`application/json`である。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `GET /list/article/{year}`
現在登録されている記事のIDのうち、その記事の作成が`{year}`年であるIDを返す。`{year}`は半角アラビア数値で記述された非負整数を受け付ける。
このエンドポイントのレスポンスは、IDの出現順序が何らかの一貫した順序付けになっているとは限らない。

#### レスポンス
* `200`: 指定された記事が (**0件以上**) 見つかった。`Content-Type`は`application/json`である。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `GET /list/article/{year}/{month}`
現在登録されている記事のIDのうち、その記事の作成が`{year}`年かつ`{month}`月であるIDを返す。`{year}`は半角アラビア数値で記述された非負整数を受け付ける。また、`{month}`は半角アラビア数字で記述された1以上12以下の非負整数を受け付ける。`{month}`が1月から9月までの場合、半角アラビア数字の0を1桁文字列の先頭にパディングする必要がある (例: 1月なら `01`)。
このエンドポイントのレスポンスは、IDの出現順序が何らかの一貫した順序付けになっているとは限らない。

#### レスポンス
* `200`: 指定された記事が (**0件以上**) 見つかった。`Content-Type`は`application/json`である。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `GET /article/{article_id}`
記事を返す。

#### レスポンス
* `200`: 指定された記事が見つかった。本文は`text/plain`である。
* `404`: 指定された記事が見つからなかった。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `POST /article/{article_id}`
記事を作成する。

#### ボディ
* 記事の本文

#### ヘッダー
* `Content-Type: text/plain`

#### レスポンス
* `200`: OK。指定された記事は作成された。
* `400`: リクエスト中の本文がおかしかった。
* `410`: すでに指定されたIDで記事が作成されている。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `PUT /article/{article_id}`
記事を更新する。

#### レスポンス
* `200`: OK。指定された記事は更新された。
* `400`: リクエスト中の本文がおかしかった。
* `404`: 指定されたIDの記事は存在しない。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `DELETE /article/{article_id}`
記事を削除する。

#### レスポンス
* `200`: OK。指定された記事は削除された。
* `404`: 指定されたIDの記事は存在しない。
* `500`: バックエンド側で予期せぬ例外が起きた。

## ライセンス
MIT ([本文](https://github.com/KisaragiEffective/toy-blog/blob/develop/LICENSE))
