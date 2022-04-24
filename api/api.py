import arrow as arrow
import fastapi
from arrow import ParserError
from fastapi import Query
from pony.orm import *
from starlette.middleware.cors import CORSMiddleware

from models import *

description = """

*BGPKIT Broker API provides lookup service for historical MRT data files.*

### Data Update Frequency and Limitation

The backend fetches the recent RouteViews and RIPE RIS MRT data files data every 5 minutes.

### Data Limitation and API Terms of Use

The source data may contain missing content in certain dates, this API should be treated as informational only and use 
with caution.

This data API is provided as a public API. If using this data, you need to agree with the BGPKIT LLC's 
Acceptable Use Agreement for public data APIs: https://bgpkit.com/aua

### About BGPKIT

BGPKIT LLC is a software consulting company that specializes on BGP data analysis (<https://bgpkit.com>). We develop and
maintain a number of open-source BGP data analysis tools, available at GitHub (<https://github.com/bgpkit>). 

If you find this data adds value to your workflow and would like to support our long-term development and 
maintenance of the software and data APIs, please consider sponsor us on GitHub at <https://github.com/sponsors/bgpkit>.
"""

app = fastapi.FastAPI(
    title="BGPKIT Broker API",
    description=description,
    version="2.0.0",
    terms_of_service="https://bgpkit.com/aua",
    contact={
        "name": "Contact",
        "url": "https://bgpkit.com",
        "email": "data@bgpkit.com"
    },
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get('/search', response_model=SearchResultModel)
async def search_mrt_files(
        ts_start: str = Query(None, description="start timestamp, in unix time or RFC3339 format"),
        ts_end: str = Query(None, description="end timestamp, in unix time or RFC3339 format"),
        project: str = Query(None, description="filter by project name, i.e. route-views or riperis"),
        collector_id: str = Query(None, description="filter by collector name, e.g. rrc00 or route-views2"),
        data_type: str = Query(None, description="rib or update"),
        page: int = Query(1, description="the page number starting from 1, default is 1, max is 100,000", gt=0),
        page_size: int = Query(100, description="the size of each page", gt=0),
):
    """
    ### MRT File Search Query

    The `/search` endpoint has the following available parameters:
    - `ts_start`: starting timestamp in string or unix timestamp format
    - `ts_end`: ending timestamp in string or unix timestamp format
    - `project`: MRT data collection project name: `routeviews` or `riperis`
    - `collector_id`: collector ID, e.g. `rrc00`, `route-views2`
    - `data_type`: type of MRT data file: `update` or `rib`
    - `page`: page number to look at, starting from 1
    - `page_limit`: number of return items per page

    ### Response

    Each API response contains a few top-level data fields:
    - `count`: the number of returned items on this call
    - `page`: the current page number
    - `page_size`: the page size
    - `error`: the error messages, `null` if call is successful
    - `data`: the list of returning MRT file meta data

    The `data` field contains a number of ROA history entries, each has the following fields:
    - `ts_start`: starting time of the file in string format
    - `ts_end`: ending time of the file in string format
    - `collector_id`: collector ID, e.g. `rrc00`, `route-views2`
    - `data_type`: type of MRT data file: `update` or `rib`
    - `url`: the URL of the file
    - `rough_size`: rough file size parsed from the archive site directly
    - `exact_size`: exact file size queried directly to the file, potentially missing (i.e. size of `0`)

    """
    with db_session:
        query = Item.select()
        if ts_start:
            try:
                start = arrow.get(ts_start).to("utc").datetime
                print(start)
                query = query.filter(lambda i: i.ts_end >= start)
            except ParserError as e:
                return SearchResultModel(error=f"failed to parse ts_start time string: {e}")
        if ts_end:
            try:
                end = arrow.get(ts_end).to("utc").datetime
                print(end)
                query = query.filter(lambda i: i.ts_start <= end)
            except ParserError as e:
                return SearchResultModel(error=f"failed to parse ts_end time string: {e}")

        if data_type:
            query = query.filter(lambda i: i.data_type == data_type)

        if project:
            if project.lower() == "route-views" or project.lower() == "routeviews" or project.lower() == "rv":
                query = query.filter(lambda i: i.collector_id.startswith("route-views"))
            elif project.lower() == "ripe-ris" or project.lower() == "riperis" or project.lower() == "ris":
                query = query.filter(lambda i: i.collector_id.startswith("rrc"))
            else:
                return SearchResultModel(error=f"unknown project {project}: use 'routeviews' or 'riperis'")

        if collector_id:
            query = query.filter(lambda i: i.collector_id == collector_id)

        query = query.order_by(Item.ts_start).page(page, page_size)

        result = [ItemModel.from_orm(p) for p in query]

    return SearchResultModel(count=len(result), page=page, page_size=page_size, data=result, error=None)
