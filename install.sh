#!/bin/bash

set -e
cargo build --release

sudo cp ./target/release/bgpkit-broker-updater /usr/local/bin/bgpkit-broker-updater
sudo cp ./deployment/full-config.json /usr/local/etc/bgpkit-broker-collectors.json