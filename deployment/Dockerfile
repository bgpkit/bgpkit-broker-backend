# select build image
FROM rust:1.62 as build

# create a new empty shell project
RUN USER=root cargo new --bin my_project
WORKDIR /my_project

# copy your source tree
COPY ./src ./src
COPY ./Cargo.toml .
COPY ./Cargo.lock .
COPY ./diesel.toml .

# build for release
RUN cargo build --release


# our final base
FROM debian:bullseye

# copy the build artifact from the build stage
COPY --from=build /my_project/target/release/bgpkit-broker-updater /usr/local/bin/bgpkit-broker-updater

RUN DEBIAN=NONINTERACTIVE apt update; apt install -y libssl-dev libpq-dev ca-certificates tzdata cron python3-pip postgresql-client; rm -rf /var/lib/apt/lists/*

COPY api/api.py /usr/local/bin/broker-api.py
COPY api/requirements.txt /tmp/requirements.txt
COPY deployment/full-config.json /usr/local/etc/bgpkit-broker-collectors.conf

COPY deployment/start_api.sh /usr/local/bin/start_api
RUN chmod +x /usr/local/bin/start_api

RUN pip3 install -r /tmp/requirements.txt


COPY deployment/update.cron /etc/cron.d/updater-crontab
RUN chmod 0644 /etc/cron.d/updater-crontab &&\
     crontab /etc/cron.d/updater-crontab

# set the startup command to run your binary
ENTRYPOINT ["bash"]
