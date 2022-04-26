#!/usr/bin/env bash

export BROKER_PG_HOST=postgres
export BROKER_PG_USER=${POSTGRES_USER}
export BROKER_PG_DB=${POSTGRES_DB}
export BROKER_PG_PASSWORD=${POSTGRES_PASSWORD}

/usr/local/bin/uvicorn --app-dir /usr/local/bin/ broker-api:app --host 0.0.0.0 --port 18888
