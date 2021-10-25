# Billboard -- BGP Data Sources

**billboard** is a Rust project aims to provide unified data sources informaion for 
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

### Billboard-scraper
1. user specify the collectors to gather information from
2. user specify scrape mode: scrape-all, or scrape-latest
3. billboard spawn tasks for scraping months of data for each collector
4. billboard backend handles saving scraped data

### Billboard-API

### Billboard-UI

## Planned Features

- [x] BGP archive data collection tool
- [x] proper logging implementation
- [ ] live (semi-realtime) data collection service
- [x] database maintaining collected information
- [ ] API service for accessing the data
- [ ] UI for accessing the data online

## Known Issues

- [ ] missing data file size parsing
- [ ] missing last_modified timestamp parsing