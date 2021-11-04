# BGPKIT Broker API

BGPKIT Broker API accesses backend database and provides API endpoints
for searching BGP data files.

## Design

`actix-web` is the web-framework of choice for bgpkit-broker-api. It
is known for speed and provides great integration with `diesel`
(our database ORM library). Other frameworks like `Rocket` could 
also be a good choice that we might explore in the future.

## API Endpoints

### `/meta`

Meta endpoints provides information about:
- [x] collectors
- [ ] kafka stream information
 
### `/search`

Search endpoint provides the searching functionality with the following
features:
- [x] search by time (UNIX timestamp)
  - `start_ts`: data file timestamp must be greater than or equals to `start_ts`
  - `end_ts`: data file timestamp must be less than or equals to `end_ts`
- [x] search by projects
  - `project`: either `riperis` or `routeviews`
- [x] search by collectors
  - `collector`: exact match of the collector name, e.g. `rrc01` or `route-views2`
- [x] search by types
  - `data_type`: either `rib` or `update`
- [x] limit the number of returned entries
- [x] order by time or reverse
  - `order`: order by timestamp, either `desc` (default) or `asc`

**Examples**:
- `/search?start_ts=X`

### `/stats`

Stats endpoint provide some informational statistics of the system:
- [x] most recent timestamp new data added per collector
  - `/stats/latest_times`: returns the latest timestamp of data files for each collector and each data type
- [x] total files in db
  - `/stats/total_count`: returns the total count of items in DB