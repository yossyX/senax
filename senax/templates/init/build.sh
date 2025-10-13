#!/bin/sh -e

# Do not modify this line. (Client)

senax model -c

# Do not modify this line. (Api)

cargo run -- gql-schema > schema.graphql
if sed --version 2>&1 | grep -q GNU; then
  sed -i '/login/d' schema.graphql
  sed -i 's/<Utc>//g' schema.graphql
else
  sed -i '' -e '/login/d' schema.graphql
  sed -i '' -e 's/<Utc>//g' schema.graphql
fi

cargo run -- migrate -t -c
senax gen-migrate auto --skip-empty --use-test-db
cargo run -- migrate -t
cargo run -- migrate --ignore-missing
senax reflect-migration-changes
cargo run -- open-api > open-api.json

codegen () {
  pid=`ps auxw | grep $1 | awk '!/grep/{print $2}'`
  if [ -n "${pid}" ]; then
    kill -s USR2 ${pid}
    sleep 20
    (cd $2; npm install; npm run codegen)
  else
    RUST_LOG=warn $1 &
    pid=$!
    sleep 20
    (cd $2; npm install; npm run codegen) || true
    kill ${pid}
  fi
}

# Do not modify this line. (Codegen)
@{-"\n"}@