# BGPKIT Broker -- BGP Data Sources

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

## LICENSE

See [LICENSE][LICENSE] file for details. 

In summary, the BGPKIT Broker backend is free for research and education usages.
For commercial usage or creating public access points, please contact us at contact@bgpkit.com.

## Built with ❤️ by BGPKIT Team

BGPKIT is a small-team start-up that focus on building the best tooling for BGP data in Rust. We have 10 years of
experience working with BGP data and believe that our work can enable our users to start keeping tracks of BGP data
on their own turf. Learn more about what services we provide at https://bgpkit.com.

<a href="https://bgpkit.com"><img src="https://bgpkit.com/Original%20Logo%20Cropped.png" alt="https://bgpkit.com/favicon.ico" width="200"/></a>
