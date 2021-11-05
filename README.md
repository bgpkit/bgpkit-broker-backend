# BGPKIT Broker Backend

BGPKIT Broker is a Rust project aims to provide unified data sources information for 
publicly available BGP data.

This repo maintains the data backend for the broker data.

## Data Sources

### [RIPE RIS raw BGP data](https://www.ripe.net/analyse/internet-measurements/routing-information-service-ris/ris-raw-data)

Data dump frequencies:
- BGP updates: every 5 minutes
- BGP table dump: every 8 hours

### [RouteViews](http://archive.routeviews.org)

Data dump frequencies:
- BGP updates: every 15 minutes
- BGP table dump: every 2 hours

## Components

### broker-updater

1. user specify the collectors to gather information from
2. user specify scrape mode: `scrape-all`, or `scrape-latest`
    - `scrape-all`: goes back to the beginning of time and gather everything form the collectors
    - `scrape-latest`: only scrapes the past and current months' data, enough to update the database for latest information
3. broker spawn tasks for scraping months of data for each collector
4. broker backend handles saving scraped data

### broker-api

The Broker API provides a REST interface for querying data.
The documentation is hosted at: https://docs.broker.bgpkit.com/

### Rust API

We also provide native Rust API to access the broker data in a more ergonomic way.
Please visit our [Rust API repository](https://github.com/bgpkit/bgpkit-broker) to learn more.

## Deployment

### Deploying with Docker

Here is a step-by-step guide for deploying BGPKIT Broker with Docker

1. Checkout the repository
2. cd into `deployment` directory
3. run `docker-compose up -d`

The initial database bootstrap phase would take about 3-5 minutes depending on your deployment environment.

After the initial bootstrap phase is done, the API service should be up and running, currently hosted at port `8080`. You can modify the port in `docker-compose.yml` file. In the mean time, a cronjob service also started, crawling collectors for recent data every 5 minutes. The frequency can be configured in the `update.cron`. It is not recommended to go more frequent than one crawl per 5 minutes.

You can check out if the API is running by running:
```bash
curl localhost:8080/v1/meta/collectors
```
It should give you all the currently indexed data collectors.

## LICENSE

See [LICENSE](LICENSE) file for details. 

In summary, the BGPKIT Broker backend is free for research and education usages.
If your institute is using our project and you feel like make us happy 🥰, please send us an email and tell us who you are and what kind of projects you are using our projects for contact@bgpkit.com.

For commercial usage or creating public access points, please contact us at contact@bgpkit.com.

## Built with ❤️ by BGPKIT Team

BGPKIT is a small-team start-up that focus on building the best tooling for BGP data in Rust. We have 10 years of
experience working with BGP data and believe that our work can enable our users to start keeping tracks of BGP data
on their own turf. Learn more about what services we provide at https://bgpkit.com.

<a href="https://bgpkit.com"><img src="https://spaces.bgpkit.org/assets/logos/wide-solid.png" width="200"/></a>
