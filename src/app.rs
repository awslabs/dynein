/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License").
 * You may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use ::serde::{Deserialize, Serialize};
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion, SdkConfig};
use aws_sdk_dynamodb::types::{AttributeDefinition, TableDescription};
use aws_types::region::Region;
use backon::ExponentialBuilder;
use log::{debug, error, info};
use serde_yaml::Error as SerdeYAMLError;
use std::convert::{TryFrom, TryInto};
use std::time::Duration;
use std::{
    collections::HashMap,
    env, error,
    fmt::{self, Formatter},
    fs,
    io::Error as IOError,
    path,
};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::control;
use super::key;
use super::util;

/* =================================================
struct / enum / const
================================================= */

const CONFIG_DIR: &str = ".dynein";
const CONFIG_PATH_ENV_VAR_NAME: &str = "DYNEIN_CONFIG_DIR";
const CONFIG_FILE_NAME: &str = "config.yml";
const CACHE_FILE_NAME: &str = "cache.yml";
const LOCAL_REGION: &str = "local";

pub enum DyneinFileType {
    ConfigFile,
    CacheFile,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableSchema {
    pub region: String,
    pub name: String,
    pub pk: key::Key,
    pub sk: Option<key::Key>,
    pub indexes: Option<Vec<IndexSchema>>,
    pub mode: util::Mode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexSchema {
    pub name: String,
    /// Type of index. i.e. GSI (Global Secondary Index) or LSI (Local Secondary Index).
    /// Use 'kind' as 'type' is a keyword in Rust.
    pub kind: IndexType,
    pub pk: key::Key,
    pub sk: Option<key::Key>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexType {
    Gsi,
    Lsi,
}

pub enum Messages {
    NoEffectiveTable,
}

impl fmt::Display for Messages {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Messages::NoEffectiveTable => "
To execute the command you must specify target table in one of following ways:
    * [RECOMMENDED] $ dy use <your_table> ... save target table to use.
    * Or, optionally you can pass --region and --table options to specify target for your commands. Refer --help for more information.
To find all tables in all regions, try:
    * $ dy ls --all-regions",
        })
    }
}

/// Config is saved at `~/.dynein/config.yml`.
/// using_region and using_table are changed when you execute `dy use` command.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub using_region: Option<String>,
    pub using_table: Option<String>,
    pub using_port: Option<u32>,
    #[serde(default)]
    pub query: QueryConfig,
    // pub cache_expiration_time: Option<i64>, // in second. default 300 (= 5 minutes)
    pub retry: Option<RetryConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RetryConfig {
    pub default: RetrySetting,
    pub batch_write_item: Option<RetrySetting>,
}

impl TryFrom<RetryConfig> for Retry {
    type Error = RetryConfigError;

    fn try_from(value: RetryConfig) -> Result<Self, Self::Error> {
        let default = value.default.try_into()?;
        let batch_write_item = match value.batch_write_item {
            Some(v) => Some(v.try_into()?),
            None => None,
        };
        Ok(Self {
            default,
            batch_write_item,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RetrySetting {
    pub initial_backoff: Option<Duration>,
    pub max_backoff: Option<Duration>,
    pub max_attempts: Option<u32>,
}

impl Default for RetrySetting {
    fn default() -> Self {
        Self {
            initial_backoff: None,
            max_backoff: None,
            max_attempts: Some(10),
        }
    }
}

#[derive(Error, Debug)]
pub enum RetryConfigError {
    #[error("max_attempts should be greater than zero")]
    MaxAttempts,
    #[error("max_backoff should be greater than zero")]
    MaxBackoff,
}
impl TryFrom<RetrySetting> for ExponentialBuilder {
    type Error = RetryConfigError;

    fn try_from(value: RetrySetting) -> Result<Self, Self::Error> {
        let mut builder = Self::default()
            .with_jitter()
            .with_factor(2.0)
            .with_min_delay(Duration::from_secs(1));

        if let Some(max_attempts) = value.max_attempts {
            if max_attempts == 0 {
                return Err(RetryConfigError::MaxAttempts);
            }
            builder = builder.with_max_times(max_attempts as usize - 1);
        }
        if let Some(max_backoff) = value.max_backoff {
            if max_backoff.is_zero() {
                return Err(RetryConfigError::MaxBackoff);
            }
            builder = builder.with_max_delay(max_backoff);
        }
        if let Some(initial_backoff) = value.initial_backoff {
            builder = builder.with_min_delay(initial_backoff);
        }
        Ok(builder)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct QueryConfig {
    #[serde(default)]
    pub strict_mode: bool,
}

/// Cache is saved at `~/.dynein/cache.yml`
/// Cache contains retrieved info of tables, and how fresh they are (cache_created_at).
/// Currently Cache struct doesn't manage freshness of each table.
/// i.e. Entire cache will be removed after cache_expiration_time in Config has passed.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Cache {
    /// cached table schema information.
    /// table schemas are stored in keys to identify the target table "<Region>/<TableName>" -- e.g. "ap-northeast-1/Employee"
    pub tables: Option<HashMap<String, TableSchema>>,
    // pub cache_updated_at: String,
    // pub cache_created_at: String,
}

#[derive(Debug, Clone)]
pub struct Retry {
    pub default: ExponentialBuilder,
    pub batch_write_item: Option<ExponentialBuilder>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub config: Option<Config>,
    pub cache: Option<Cache>,
    pub overwritten_region: Option<Region>, // --region option
    pub overwritten_table_name: Option<String>, // --table option
    pub overwritten_port: Option<u32>,      // --port option
    pub output: Option<String>,
    pub should_strict_for_query: Option<bool>,
    pub retry: Option<Retry>,
}

/*
 When region/table info is given by command line arguments (--region/--table),
 Context object has overwritten_region/overwritten_table_name values. Implemented in main.rs.
 Overwritten information is retrieved with `effective_*` functions as 1st priority.
*/
impl Context {
    pub fn new(
        region: Option<String>,
        port: Option<u32>,
        table: Option<String>,
    ) -> Result<Context, DyneinConfigError> {
        let config = load_or_touch_config_file(true)?;
        let retry = match &config.retry {
            Some(retry) => Some(Retry::try_from(retry.clone()).map_err(|e| {
                DyneinConfigError::Content(DyneinConfigContentError::RetryConfig(e))
            })?),
            None => None,
        };
        Ok(Context {
            config: Some(config),
            cache: Some(load_or_touch_cache_file(true)?),
            overwritten_region: region_from_str(region, port),
            overwritten_table_name: table,
            overwritten_port: port,
            output: None,
            should_strict_for_query: None,
            retry,
        })
    }

    pub async fn effective_sdk_config(&self) -> SdkConfig {
        let region = self.effective_region();
        let region_name = region.as_ref();

        self.effective_sdk_config_with_region(region_name).await
    }

    pub async fn effective_sdk_config_with_region(&self, region_name: &str) -> SdkConfig {
        let sdk_region = Region::new(region_name.to_owned());

        let provider = RegionProviderChain::first_try(sdk_region);
        aws_config::defaults(BehaviorVersion::v2024_03_28())
            .region(provider)
            .load()
            .await
    }

    pub fn effective_region(&self) -> Region {
        // if region is overwritten by --region comamnd, use it.
        if let Some(ow_region) = &self.overwritten_region {
            return ow_region.to_owned();
        };

        // next, if there's an `using_region` field in the config file, use it.
        if let Some(using_region_name_in_config) =
            &self.config.to_owned().and_then(|x| x.using_region)
        {
            return region_from_str(
                Some(using_region_name_in_config.to_owned()),
                Some(self.effective_port()),
            ) // Option<Region>
            .expect("Region name in the config file is invalid.");
        };

        // otherwise, come down to "default region" of your environment.
        // e.g. region set via AWS CLI (check: $ aws configure get region), or environment variable `AWS_DEFAULT_REGION`.
        //      ref: https://docs.rs/rusoto_signature/0.42.0/src/rusoto_signature/region.rs.html#282-290
        //      ref: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html
        // TODO: fix
        Region::from_static("us-east-1")
    }

    pub fn effective_table_name(&self) -> String {
        // if table is overwritten by --table option, use it.
        if let Some(ow_table_name) = &self.overwritten_table_name {
            return ow_table_name.to_owned();
        };
        // otherwise, retrieve an `using_table` from config file.
        self.to_owned()
            .config
            .and_then(|x| x.using_table)
            .unwrap_or_else(|| {
                // if both --option nor config file are not available, raise error and exit the command.
                error!("{}", Messages::NoEffectiveTable);
                std::process::exit(1)
            })
    }

    pub fn effective_port(&self) -> u32 {
        if let Some(ow_port) = &self.overwritten_port {
            return ow_port.to_owned();
        };

        if let Some(using_port_in_config) = &self.config.to_owned().and_then(|x| x.using_port) {
            return using_port_in_config.to_owned();
        };

        8000
    }

    pub fn effective_cache_key(&self) -> String {
        format!(
            "{}/{}",
            &self.effective_region().as_ref(),
            &self.effective_table_name()
        )
    }

    pub fn cached_using_table_schema(&self) -> Option<TableSchema> {
        // return None if table name is not specified in both config and option.
        if self.overwritten_table_name.is_none() {
            match self.config.to_owned() {
                Some(c) => c.using_table?,
                None => return None,
            };
        }

        let cached_tables: HashMap<String, TableSchema> =
            match self.cache.to_owned().and_then(|c| c.tables) {
                Some(cts) => cts,
                None => return None, // return None for this "cached_using_table_schema" function
            };
        let found_table_schema: Option<&TableSchema> =
            cached_tables.get(&self.effective_cache_key());
        // NOTE: HashMap's `get` returns a reference to the value / (&self, k: &Q) -> Option<&V>
        found_table_schema.map(|schema| schema.to_owned())
    }

    pub fn with_region(mut self, ec2_region: &str) -> Self {
        self.overwritten_region = Some(Region::new(ec2_region.to_owned()));
        self
    }

    pub fn with_table(mut self, table: &str) -> Self {
        self.overwritten_table_name = Some(table.to_owned());
        self
    }

    pub fn should_strict_for_query(&self) -> bool {
        self.should_strict_for_query
            .unwrap_or_else(|| self.config.as_ref().map_or(false, |c| c.query.strict_mode))
    }

    pub fn is_local(&self) -> bool {
        let region = self.effective_region();
        region.as_ref() == LOCAL_REGION
    }
}

#[derive(Error, Debug)]
pub enum DyneinConfigContentError {
    #[error("retry config error ")]
    RetryConfig(#[from] RetryConfigError),
}

// FYI: https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/wrap_error.html
#[derive(Debug)]
pub enum DyneinConfigError {
    IO(IOError),
    Yaml(SerdeYAMLError),
    HomeDir,
    Content(DyneinConfigContentError),
}

impl fmt::Display for DyneinConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DyneinConfigError::IO(ref e) => e.fmt(f),
            DyneinConfigError::Yaml(ref e) => e.fmt(f),
            DyneinConfigError::HomeDir => write!(f, "failed to find Home directory"),
            DyneinConfigError::Content(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for DyneinConfigError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            // The cause is the underlying implementation error type. Is implicitly cast to the trait object `&error::Error`.
            // This works because the underlying type already implements the `Error` trait.
            DyneinConfigError::IO(ref e) => Some(e),
            DyneinConfigError::Yaml(ref e) => Some(e),
            DyneinConfigError::HomeDir => None,
            DyneinConfigError::Content(ref e) => Some(e),
        }
    }
}

// Implement the conversion from existing error like `serde_yaml::Error` to `DyneinConfigError`.
// This will be automatically called by `?` if underlying errors needs to be converted into a `DyneinConfigError`.
impl From<IOError> for DyneinConfigError {
    fn from(e: IOError) -> Self {
        Self::IO(e)
    }
}
impl From<SerdeYAMLError> for DyneinConfigError {
    fn from(e: SerdeYAMLError) -> Self {
        Self::Yaml(e)
    }
}

/* =================================================
Public functions
================================================= */

// Receives given --region option string, including "local", return Region struct.
pub fn region_from_str(s: Option<String>, p: Option<u32>) -> Option<Region> {
    let port = p.unwrap_or(8000);
    match s.as_deref() {
        Some(LOCAL_REGION) => Some(region_dynamodb_local(port)),
        Some(x) => Some(Region::new(x.to_owned())), // convert Result<T, E> into Option<T>
        None => None,
    }
}

/// Loads dynein config file (YAML format) and return config struct as a result.
/// Creates the file with default if the file couldn't be found.
pub fn load_or_touch_config_file(first_try: bool) -> Result<Config, DyneinConfigError> {
    let path = retrieve_dynein_file_path(DyneinFileType::ConfigFile)?;
    debug!("Loading Config File: {}", path);

    match fs::read_to_string(&path) {
        Ok(_str) => {
            let config: Config = serde_yaml::from_str(&_str)?;
            debug!("Loaded current config: {:?}", config);
            Ok(config)
        }
        Err(e) => {
            if !first_try {
                return Err(DyneinConfigError::from(e));
            };
            info!(
                "Config file doesn't exist in the path, hence creating a blank file: {}",
                e
            );
            let yaml_string = serde_yaml::to_string(&Config {
                ..Default::default()
            })
            .unwrap();

            write_dynein_file(DyneinFileType::ConfigFile, yaml_string)?;
            load_or_touch_config_file(false) // set fisrt_try flag to false in order to avoid infinite loop.
        }
    }
}

/// Loads dynein cache file (YAML format) and return Cache struct as a result.
/// Creates the file with default if the file couldn't be found.
pub fn load_or_touch_cache_file(first_try: bool) -> Result<Cache, DyneinConfigError> {
    let path = retrieve_dynein_file_path(DyneinFileType::CacheFile)?;
    debug!("Loading Cache File: {}", path);

    match fs::read_to_string(&path) {
        Ok(_str) => {
            let cache: Cache = serde_yaml::from_str(&_str)?;
            debug!("Loaded current cache: {:?}", cache);
            Ok(cache)
        }
        Err(e) => {
            if !first_try {
                return Err(DyneinConfigError::from(e));
            };
            info!(
                "Cache file doesn't exist in the path, hence creating a blank file: {}",
                e
            );
            let yaml_string = serde_yaml::to_string(&Cache {
                ..Default::default()
            })?;

            write_dynein_file(DyneinFileType::CacheFile, yaml_string)?;
            load_or_touch_cache_file(false) // set fisrt_try flag to false in order to avoid infinite loop.
        }
    }
}

/// You can use a table in two syntaxes:
///   $ dy use --table mytable
///   or
///   $ dy use mytable
pub async fn use_table(
    cx: &mut Context,
    positional_arg_table_name: Option<String>,
) -> Result<(), DyneinConfigError> {
    // When context has "overwritten_table_name". i.e. you passed --table (-t) option.
    // When you didn't pass --table option, check if you specified target table name directly, instead of --table option.
    let target_table: Option<&String> = cx
        .overwritten_table_name
        .as_ref()
        .or(positional_arg_table_name.as_ref());
    match target_table {
        Some(tbl) => {
            debug!("describing the table: {}", tbl);
            let tbl = tbl.clone();
            let desc: TableDescription = control::describe_table_api(cx, tbl.clone()).await;
            save_using_target(cx, desc)?;
            println!("Now you're using the table '{}' ({}).", tbl, &cx.effective_region().as_ref());
        },
        None => bye(1, "You have to specify a table. How to use (1). 'dy use --table mytable', or (2) 'dy use mytable'."),
    };

    Ok(())
}

/// Inserts specified table description into cache file.
pub fn insert_to_table_cache(
    cx: &Context,
    desc: TableDescription,
) -> Result<(), DyneinConfigError> {
    let table_name: String = desc
        .table_name
        .clone()
        .expect("desc should have table name");
    let region: Region = cx.effective_region();
    debug!(
        "Under the region '{}', trying to save table schema of '{}'",
        &region.as_ref(),
        &table_name
    );

    // retrieve current cache from Context and update target table desc.
    // key to save the table desc is "<RegionName>/<TableName>" -- e.g. "us-west-2/app_data"
    let mut cache: Cache = cx.clone().cache.expect("cx should have cache");
    let cache_key = format!("{}/{}", region.as_ref(), table_name);

    let mut table_schema_hashmap: HashMap<String, TableSchema> = match cache.tables {
        Some(ts) => ts,
        None => HashMap::<String, TableSchema>::new(),
    };
    debug!(
        "table schema cache before insert: {:#?}",
        table_schema_hashmap
    );

    table_schema_hashmap.insert(
        cache_key,
        TableSchema {
            region: String::from(region.as_ref()),
            name: table_name,
            pk: key::typed_key("HASH", &desc).expect("pk should exist"),
            sk: key::typed_key("RANGE", &desc),
            indexes: index_schemas(&desc),
            mode: util::extract_mode(&desc.billing_mode_summary),
        },
    );
    cache.tables = Some(table_schema_hashmap);

    // write to cache file
    let cache_yaml_string = serde_yaml::to_string(&cache)?;
    debug!(
        "this YAML will be written to the cache file: {:#?}",
        &cache_yaml_string
    );
    write_dynein_file(DyneinFileType::CacheFile, cache_yaml_string)?;

    Ok(())
}

/// Physicall remove config and cache file.
pub fn remove_dynein_files() -> Result<(), DyneinConfigError> {
    fs::remove_file(retrieve_dynein_file_path(DyneinFileType::ConfigFile)?)?;
    fs::remove_file(retrieve_dynein_file_path(DyneinFileType::CacheFile)?)?;
    Ok(())
}

// If you explicitly specify target table by `--table/-t` option, this function executes DescribeTable API to gather table schema info.
// Otherwise, load table schema info from config file.
// fn table_schema(region: &Region, config: &config::Config, table_overwritten: Option<String>) -> TableSchema {
pub async fn table_schema(cx: &Context) -> TableSchema {
    match cx.overwritten_table_name.to_owned() {
        // It's possible that users pass --table without calling `dy use` for any table. Thus collect all data from DescribeTable results.
        Some(table_name) => {
            // TODO: reduce # of DescribeTable API calls. table_schema function is called every time you do something.
            let desc: TableDescription = control::describe_table_api(
                cx, table_name, /* should be equal to 'cx.effective_table_name()' */
            )
            .await;

            TableSchema {
                region: String::from(cx.effective_region().as_ref()),
                name: desc.clone().table_name.unwrap(),
                pk: key::typed_key("HASH", &desc).expect("pk should exist"),
                sk: key::typed_key("RANGE", &desc),
                indexes: index_schemas(&desc),
                mode: util::extract_mode(&desc.billing_mode_summary),
            }
        }
        None => {
            // simply maps config data into TableSchema struct.
            debug!("current context {:#?}", cx);
            let cache: Cache = cx.clone().cache.expect("Cache should exist in context"); // can refactor here using and_then
            let cached_tables: HashMap<String, TableSchema> = cache.tables.unwrap_or_else(|| {
                error!("{}", Messages::NoEffectiveTable);
                std::process::exit(1)
            });
            let schema_from_cache: Option<TableSchema> = cached_tables
                .get(&cx.effective_cache_key())
                .map(|x| x.to_owned());
            schema_from_cache.unwrap_or_else(|| {
                error!("{}", Messages::NoEffectiveTable);
                std::process::exit(1)
            })
        }
    }
}

pub fn index_schemas(desc: &TableDescription) -> Option<Vec<IndexSchema>> {
    let attr_defs: &Vec<AttributeDefinition> = &desc.clone().attribute_definitions.unwrap();

    let mut indexes: Vec<IndexSchema> = vec![];

    if let Some(gsis) = desc.clone().global_secondary_indexes {
        for gsi in gsis {
            indexes.push(IndexSchema {
                name: gsi.index_name.unwrap(),
                kind: IndexType::Gsi,
                pk: key::typed_key_for_schema("HASH", &gsi.key_schema.clone().unwrap(), attr_defs)
                    .expect("pk should exist"),
                sk: key::typed_key_for_schema("RANGE", &gsi.key_schema.unwrap(), attr_defs),
            });
        }
    };

    if let Some(lsis) = desc.clone().local_secondary_indexes {
        for lsi in lsis {
            indexes.push(IndexSchema {
                name: lsi.index_name.unwrap(),
                kind: IndexType::Lsi,
                pk: key::typed_key_for_schema("HASH", &lsi.key_schema.clone().unwrap(), attr_defs)
                    .expect("pk should exist"),
                sk: key::typed_key_for_schema("RANGE", &lsi.key_schema.unwrap(), attr_defs),
            });
        }
    };

    if indexes.is_empty() {
        None
    } else {
        Some(indexes)
    }
}

pub fn bye(code: i32, msg: &str) -> ! {
    println!("{}", msg);
    std::process::exit(code);
}

/* =================================================
Private functions
================================================= */

fn region_dynamodb_local(port: u32) -> Region {
    let endpoint_url = format!("http://localhost:{}", port);
    debug!(
        "setting DynamoDB Local '{}' as target region.",
        &endpoint_url
    );
    // TODO: fix
    Region::from_static(LOCAL_REGION)
}

fn retrieve_dynein_file_path(file_type: DyneinFileType) -> Result<String, DyneinConfigError> {
    let filename = match file_type {
        DyneinFileType::ConfigFile => CONFIG_FILE_NAME,
        DyneinFileType::CacheFile => CACHE_FILE_NAME,
    };

    Ok(format!("{}/{}", retrieve_or_create_dynein_dir()?, filename))
}

fn retrieve_or_create_dynein_dir() -> Result<String, DyneinConfigError> {
    let full_path = env::var(CONFIG_PATH_ENV_VAR_NAME).unwrap_or(
        dirs::home_dir()
            .ok_or(DyneinConfigError::HomeDir)?
            .to_str()
            .ok_or(DyneinConfigError::HomeDir)?
            .to_string(),
    );

    let dir = path::Path::new(&full_path).join(CONFIG_DIR);

    if !dir.exists() {
        debug!("Creating dynein config directory: {}", dir.display());
        fs::create_dir_all(&dir)?;
    };

    Ok(dir.to_str().ok_or(DyneinConfigError::HomeDir)?.to_string())
}

/// This function updates `using_region` and `using_table` in config.yml,
/// and at the same time inserts TableDescription of the target table into cache.yml.
fn save_using_target(cx: &mut Context, desc: TableDescription) -> Result<(), DyneinConfigError> {
    let table_name: String = desc
        .table_name
        .clone()
        .expect("desc should have table name");

    let port: u32 = cx.effective_port();

    // retrieve current config from Context and update "using target".
    let region = Some(String::from(cx.effective_region().as_ref()));
    let config = cx.config.as_mut().expect("cx should have config");
    config.using_region = region;
    config.using_table = Some(table_name);
    config.using_port = Some(port);
    debug!("config file will be updated with: {:?}", config);

    // write to config file
    let config_yaml_string = serde_yaml::to_string(config)?;
    write_dynein_file(DyneinFileType::ConfigFile, config_yaml_string)?;

    // save target table info into cache.
    insert_to_table_cache(cx, desc)?;

    Ok(())
}

fn write_dynein_file(file_type: DyneinFileType, content: String) -> Result<(), DyneinConfigError> {
    let temp_file = NamedTempFile::new_in(retrieve_or_create_dynein_dir()?)?;
    let temp_path = temp_file.path();

    fs::write(temp_path, content)?;
    fs::rename(temp_path, retrieve_dynein_file_path(file_type)?)?;

    Ok(())
}

/* =================================================
Unit Tests
================================================= */

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;
    use std::error::Error;

    #[test]
    fn test_context_functions() -> Result<(), Box<dyn Error>> {
        let cx1 = Context {
            config: None,
            cache: None,
            overwritten_region: None,
            overwritten_table_name: None,
            overwritten_port: None,
            output: None,
            should_strict_for_query: None,
            retry: None,
        };
        assert_eq!(cx1.effective_region(), Region::from_static("us-east-1"));
        // cx1.effective_table_name(); ... exit(1)

        let cx2 = Context {
            config: Some(Config {
                using_region: Some(String::from("ap-northeast-1")),
                using_table: Some(String::from("cfgtbl")),
                using_port: Some(8000),
                query: QueryConfig { strict_mode: false },
                retry: Some(RetryConfig::default()),
            }),
            cache: None,
            overwritten_region: None,
            overwritten_table_name: None,
            overwritten_port: None,
            output: None,
            should_strict_for_query: None,
            retry: Some(RetryConfig::default().try_into()?),
        };
        assert_eq!(
            cx2.effective_region(),
            Region::from_static("ap-northeast-1")
        );
        assert_eq!(cx2.effective_table_name(), String::from("cfgtbl"));

        let cx3 = Context {
            overwritten_region: Some(Region::from_static("us-east-1")), // --region us-east-1
            overwritten_table_name: Some(String::from("argtbl")),       // --table argtbl
            ..cx2.clone()
        };
        assert_eq!(cx3.effective_region(), Region::from_static("us-east-1"));
        assert_eq!(cx3.effective_table_name(), String::from("argtbl"));

        let cx4 = Context {
            overwritten_region: Some(Region::from_static("us-east-1")), // --region us-east-1
            ..cx2.clone()
        };
        assert_eq!(cx4.effective_region(), Region::from_static("us-east-1"));
        assert_eq!(cx4.effective_table_name(), String::from("cfgtbl"));

        let cx5 = Context {
            overwritten_table_name: Some(String::from("argtbl")), // --table argtbl
            ..cx2.clone()
        };
        assert_eq!(
            cx5.effective_region(),
            Region::from_static("ap-northeast-1")
        );
        assert_eq!(cx5.effective_table_name(), String::from("argtbl"));

        Ok(())
    }

    #[test]
    fn test_retry_setting_success() {
        let config1 = RetrySetting::default();
        let actual = ExponentialBuilder::try_from(config1).unwrap();
        let expected = ExponentialBuilder::default()
            .with_min_delay(Duration::from_secs(1))
            .with_jitter()
            .with_factor(2.0)
            .with_max_times(9);
        assert_eq!(format!("{:?}", actual), format!("{:?}", expected));

        let config2 = RetrySetting {
            initial_backoff: Some(Duration::from_secs(1)),
            max_backoff: Some(Duration::from_secs(100)),
            max_attempts: Some(20),
        };
        let actual = ExponentialBuilder::try_from(config2).unwrap();
        let expected = ExponentialBuilder::default()
            .with_jitter()
            .with_factor(2.0)
            .with_min_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(100))
            .with_max_times(19);
        assert_eq!(format!("{:?}", actual), format!("{:?}", expected));
    }

    #[test]
    fn test_retry_setting_error() {
        let config = RetrySetting {
            max_attempts: Some(0),
            ..Default::default()
        };
        match ExponentialBuilder::try_from(config).unwrap_err() {
            RetryConfigError::MaxAttempts => {}
            _ => unreachable!("unexpected error"),
        }

        let config = RetrySetting {
            max_backoff: Some(Duration::new(0, 0)),
            ..Default::default()
        };
        match ExponentialBuilder::try_from(config).unwrap_err() {
            RetryConfigError::MaxBackoff => {}
            _ => unreachable!("unexpected error"),
        }
    }
}
