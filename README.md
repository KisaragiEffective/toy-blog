# toy-blog
## 概要
[`KisaragiEffective`](https://github.com/KisaragiEffective/) が開発する実験用のブログ。

## How to run
```sh
cargo run -- \
  --bearer-token=INSERT_YOUR_OWN_TOKEN \
  --http-host=127.0.0.1 \
  --http-port=8080 \
  --telnet-host=127.0.0.1 \
  --telnet-port=12345
```

Cloudflareのトンネルを使っている場合、`--cloudflare`スイッチを付け足すこと。

## 永続化
全てのデータはJSONで格納される。

データは`data/`ディレクトリ以下に保存される。

### ディレクトリ構造
* `data/`
  * `articles.json`
  * `cors_setting.json`

### `articles.json`
記事のデータを格納する。
* `data: map`
  * key: 記事ID
  * value
    * `created_at: date`: 作成日時
    * `updated_at: date`: 更新日時
    * `content: string` : 記事の本文

実装上の注: `GET /article/{article_id}`の応答速度を向上させるためにmapを用いている。

### `cors_setting.json`
CORSアクセスが許可されるプロトコル付きのFQDNを記述する。
* `_: array`
   * element
     * `_: string` - プロトコル付きのFQDN。例えば、`https://my-frontend.example.com`

## API
APIのエンドポイントのベースは`http://{YOUR_DOMAIN}/api`である。

### `GET /articles`
現在登録されている記事を配列形式で全て返す。

#### レスポンス
* `200`: 指定された記事が見つかった。本文はJSONである。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `GET /article/{article_id}`
記事を返す。

#### レスポンス
* `200`: 指定された記事が見つかった。本文はプレーン・テキストである。
* `404`: 指定された記事が見つからなかった。
* `500`: バックエンド側で予期せぬ例外が起きた。

### `POST /article/{article_id}`
記事を作成する。

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
