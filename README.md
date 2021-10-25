# BGPKIT Broker -- BGP Data Sources

BGPKIT Broker is a Rust project aims to provide unified data sources information for 
publicly available BGP data.

## Data Sources

### [RIPE RIS raw BGP data](https://www.ripe.net/analyse/internet-measurements/routing-information-service-ris/ris-raw-data)

Data dump frequencies:
- BGP updates: every 5 minutes
- BGP table dump: every 8 hours

### [RouteViews](http://archive.routeviews.org)

Data dump frequencies:
- BGP updates: every 15 minutes
- BGP table dump: every 2 hours

## Workflow

### broker-scraper
1. user specify the collectors to gather information from
2. user specify scrape mode: scrape-all, or scrape-latest
3. billboard spawn tasks for scraping months of data for each collector
4. billboard backend handles saving scraped data