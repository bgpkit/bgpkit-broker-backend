# BGPKIT Broker Deployment

## Current Production Deployment

### Production Components

- main database: PostgreSQL 14 on FreeBSD
- content updater: the following tasks are executed every 5 minutes
  - run `bgpkit-broker-updater`, pushing changes to PostgreSQL database
  - generate materialized view for `latest_times`
  - *experimental*: pushing `latest_times` view content to Cloudflare KV
    - content served using Worker (see script below) at https://broker-latest.bgpkit.workers.dev/
- RESTful API: Running as a RC service on FreeBSD
  - automatically start with the system
- PostgreSQL backups (scripts see backup section below)
  - daily backup (keeping only one)
    - running as root on the PostgreSQL VM
    - backup file pushed to https://data.bgpkit.com/broker/bgpkit_broker_prod_postgres.gz
  - weekly backup (keeping only one)
    - backup to DigitalOcean Spaces weekly
    - running as normal user using `s3cmd`
    - available at https://spaces.bgpkit.org/broker/broker-database-dump-v2.gz

#### API FreeBSD RC Script

File at `/etc/rc.d/bgpkit_broker`:
```bash
#!/bin/sh

. /etc/rc.subr

name="bgpkit_broker"
rcvar=${name}_enable
pidfile="/var/run/${name}.pid"

export BROKER_PG_HOST="*****"
export BROKER_PG_USER="*****"
export BROKER_PG_DB="*****"

command="/usr/sbin/daemon"
command_args="-r -t bgpkit_broker -S -P ${pidfile} /usr/local/bin/uvicorn --app-dir /usr/local/bin/ broker-api:app --host 0.0.0.0 --port 18888 --root-path '/v2'"

load_rc_config $name
run_rc_command "$1"
```

The following line is added to `/etc/rc.conf`
```text
bgpkit_broker_enable="YES"
```

#### Cloudflare KV and Workers for Live View
Cloudflare Worker code for serving the latest view:
```js
addEventListener('fetch', event => {
  event.respondWith(handleRequest(event.request))
})

async function handleRequest(request) {
  const resp = await BGPKIT_BROKER.get("latest", { type: "json" });

  return new Response(JSON.stringify(resp), {
    headers: {
      'Content-type': 'application/json',
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET",
      "Access-Control-Max-Age": "86400",
    }
  })
}
```

#### PostgreSQL Backup Script

The daily backup script is as follows:
```bash
#!/usr/local/bin/bash
set -e
DUMP_FILE_TMP=/data/bgpkit/public/broker/bgpkit_broker_prod_postgres_tmp.gz
DUMP_FILE=/data/bgpkit/public/broker/bgpkit_broker_prod_postgres.gz
pg_dump -O -x bgpkit_prod -U postgres | gzip > $DUMP_FILE_TMP && mv $DUMP_FILE_TMP $DUMP_FILE
```
The files in directory `/data/bgpkit/public/broker/` is mapped to https://data.bgpkit.com/broker/

The weekly backup script is as follows: 
```bash
#!/usr/local/bin/bash

set -e 

DUMP_FILE=/home/mingwei/broker-database-dump.gz

pg_dump -U postgres -O -x bgpkit_prod | gzip > $DUMP_FILE
s3cmd put --acl-public /home/mingwei/broker-database-dump.gz s3://bgpkit-data/broker/broker-database-dump-v2.gz
rm $DUMP_FILE
```
The weekly backup runs as normal user with DigitalOcean credential previously configured.

## Dockerize Environment

### `docker-compose` Quick Start

Go to the root directory and run the following command
```bash
docker-compose -f deployment/docker-compose.yml up
```
