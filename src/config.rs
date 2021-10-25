use serde::{Deserialize, Serialize};
use crate::models::Collector as BCollector;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Project {
    name: String,
    collectors: Vec<Collector>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Collector {
    id: String,
    url: String,
}

impl Config {
    pub fn to_collectors(self: &Self) -> Vec<BCollector> {
        let mut collectors = vec![];
        for project in &self.projects {
            assert!(["routeviews", "riperis"].contains(&project.name.as_str()));

            let cs: Vec<BCollector> = project.collectors.iter().map(|c| BCollector{
                id: c.id.clone(),
                project: project.name.clone(),
                url: c.url.clone()
            }).collect();
            collectors.extend(cs);
        }
        collectors
    }
}
