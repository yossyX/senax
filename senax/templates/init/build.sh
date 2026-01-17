#!/bin/sh -e

# Do not modify this line. (Client)

# Do not modify this line. (Api)

senax model -c
cargo check

cargo run -p db -- migrate -t -c
senax gen-migrate auto --skip-empty --use-test-db
cargo run -p db -- migrate -t
cargo run -p db -- migrate --ignore-missing
senax reflect-migration-changes
# cargo run --features=seed_schema -p db -- gen-seed-schema

schema() {
  pkg="$1"
  ts_dir="$2"

  cargo run -p "$pkg" -- gen-gql-schema $ts_dir
  cargo run -p "$pkg" -- gql-schema > "$pkg/schema.graphql"

  if sed --version 2>&1 | grep -q GNU; then
    sed -i '/login/d' "$pkg/schema.graphql"
    sed -i 's/<Utc>//g' "$pkg/schema.graphql"
  else
    sed -i '' -e '/login/d' "$pkg/schema.graphql"
    sed -i '' -e 's/<Utc>//g' "$pkg/schema.graphql"
  fi

  cargo run -p "$pkg" -- open-api > "$pkg/open-api.json"
}

codegen () {
  pid=`ps auxw | grep $1 | awk '!/grep/{print $2}'`
  if [ -n "${pid}" ]; then
    kill -s USR2 ${pid}
    sleep 1
    (cd $2; npm install; npm run codegen)
  else
    RUST_LOG=warn $1 &
    pid=$!
    sleep 1
    (cd $2; npm install; npm run codegen) || true
    kill ${pid}
  fi
}

# Do not modify this line. (Schema)

# Do not modify this line. (Codegen)
@{-"\n"}@