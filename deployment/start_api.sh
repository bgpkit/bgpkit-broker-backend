#!/usr/bin/env bash

# the following environment variables are required
# POSTGRES_HOST
# POSTGRES_PASSWORD
# POSTGRES_USER
# POSTGRES_DB

/usr/local/bin/uvicorn --app-dir /usr/local/bin/ broker-api:app --host 0.0.0.0 --port 18888
