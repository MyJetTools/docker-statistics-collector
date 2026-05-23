use std::collections::{BTreeMap, HashMap};

use crate::models::ContainerJsonModel;

use super::{DataCache, HostMemSnapshot};

pub struct DataCacheByEnv {
    pub envs: BTreeMap<String, DataCache>,
}

impl DataCacheByEnv {
    pub fn new() -> Self {
        Self {
            envs: BTreeMap::new(),
        }
    }

    pub fn update_from_master(
        &mut self,
        env: &str,
        containers_by_instance: BTreeMap<String, Vec<ContainerJsonModel>>,
        host_mem_by_instance: HashMap<String, HostMemSnapshot>,
        master_url: String,
    ) {
        if !self.envs.contains_key(env) {
            self.envs.insert(env.to_string(), DataCache::new());
        }

        self.envs.get_mut(env).unwrap().update_from_master(
            containers_by_instance,
            host_mem_by_instance,
            master_url,
        );
    }
}
