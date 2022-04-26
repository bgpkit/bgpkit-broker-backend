# select build image
FROM rust:1.56 as build

# create a new empty shell project
RUN USER=root cargo new --bin my_project
WORKDIR /my_project

# copy your source tree
COPY ./src ./src
COPY ./api ./api
COPY ./Cargo.toml .
COPY ./Cargo.lock .
COPY ./diesel.toml .

# build for release
RUN cargo build --release

# our final base
FROM debian:bullseye

RUN DEBIAN=NONINTERACTIVE apt update; apt install -y libssl-dev libpq-dev ca-certificates tzdata cron; rm -rf /var/lib/apt/lists/*

# copy the build artifact from the build stage
COPY --from=build /my_project/api/api.py /usr/local/bin/broker-api.py
COPY --from=build /my_project/target/release/bgpkit-broker-updater /usr/local/bin/bgpkit-broker-updater

COPY deployment/update.cron /etc/cron.d/updater-crontab
RUN chmod 0644 /etc/cron.d/updater-crontab &&\
    crontab /etc/cron.d/updater-crontab

COPY deployment/full-config.json /usr/local/etc/bgpkit-broker-collectors.conf

# set the startup command to run your binary
ENTRYPOINT ["bgpkit-broker-api"]
