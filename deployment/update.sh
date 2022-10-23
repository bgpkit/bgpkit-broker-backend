#!/usr/bin/env bash

set -e

if [[ ! -z "${KAFKA_BROKER}" ]]; then
  KAFKA_OPTS="--kafka-broker ${KAFKA_BROKER} --kafka-topic ${KAFKA_TOPIC}"
else
  KAFKA_OPTS=""
fi

RUST_LOG=bgpkit_broker_backend /usr/local/bin/bgpkit-broker-updater -c /usr/local/etc/bgpkit-broker-collectors.conf --mode latest -d ${DATABASE_URL} ${KAFKA_OPTS} # 2>/tmp/bgpkit-broker-updater.log
psql ${DATABASE_URL} -c "REFRESH MATERIALIZED VIEW latest_times" # > /dev/null
