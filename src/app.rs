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
use log::{debug, error, info};
use rusoto_core::Region;
use rusoto_dynamodb::*;
use serde_yaml::Error as SerdeYAMLError;
use std::{
    collections::HashMap,
    error,
    fmt::{self, Display, Error as FmtError, Formatter},
    fs,
    io::Error as IOError,
    path,
    str::FromStr,
};

use super::control;

/* =================================================
struct / enum / const
================================================= */

const CONFIG_DIR: &str = ".dynein";
const CONFIG_FILE_NAME: &str = "config.yml";
const CACHE_FILE_NAME: &str = "cache.yml";

pub enum DyneinFileType {
    ConfigFile,
    CacheFile,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableSchema {
    pub region: String,
    pub name: String,
    pub pk: Key,
    pub sk: Option<Key>,
    pub indexes: Option<Vec<IndexSchema>>,
    pub mode: control::Mode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexSchema {
    pub name: String,
    /// Type of index. i.e. GSI (Global Secondary Index) or LSI (Local Secondary Index).
    /// Use 'kind' as 'type' is a keyword in Rust.
    pub kind: IndexType,
    pub pk: Key,
    pub sk: Option<Key>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Key {
    pub name: String,
    /// Data type of the primary key. i.e. "S" (String), "N" (Number), or "B" (Binary).
    /// Use 'kind' as 'type' is a keyword in Rust.
    pub kind: KeyType,
}

impl Key {
    /// return String with "<pk name> (<pk type>)", e.g. "myPk (S)". Used in desc command outputs.
    pub fn display(&self) -> String {
        format!("{} ({})", self.name, self.kind)
    }
}

/// Restrict acceptable DynamoDB data types for primary keys.
///     enum witn methods/FromStr ref: https://docs.rs/rusoto_signature/0.42.0/src/rusoto_signature/region.rs.html#226-258
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum KeyType {
    S,
    N,
    B,
}

/// implement Display for KeyType to simply print a single letter "S", "N", or "B".
impl Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                KeyType::S => "S",
                KeyType::N => "N",
                KeyType::B => "B",
            }
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseKeyTypeError {
    message: String,
}

impl std::error::Error for ParseKeyTypeError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl Display for ParseKeyTypeError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.message)
    }
}

impl ParseKeyTypeError {
    /// Parses a region given as a string literal into a type `KeyType'
    pub fn new(input: &str) -> Self {
        ParseKeyTypeError {
            message: format!("Not a valid DynamoDB primary key type: {}", input),
        }
    }
}

impl FromStr for KeyType {
    type Err = ParseKeyTypeError;

    fn from_str(s: &str) -> Result<KeyType, ParseKeyTypeError> {
        match s {
            "S" => Ok(KeyType::S),
            "N" => Ok(KeyType::N),
            "B" => Ok(KeyType::B),
            x => Err(ParseKeyTypeError::new(x)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexType {
    GSI,
    LSI,
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
    // pub cache_expiration_time: Option<i64>, // in second. default 300 (= 5 minutes)
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
pub struct Context {
    pub region: Option<Region>,
    pub config: Option<Config>,
    pub cache: Option<Cache>,
    pub overwritten_region: Option<Region>, // --region option
    pub overwritten_table_name: Option<String>, // --table option
    pub output: Option<String>,
}

/*
 When region/table info is given by command line arguments (--region/--table),
 Context object has overwritten_region/overwritten_table_name values. Implemented in main.rs.
 Overwritten information is retrieved with `effective_*` functions as 1st priority.
*/
impl Context {
    pub fn effective_region(&self) -> Region {
        // if region is overwritten by --region comamnd, use it.
        if let Some(ow_region) = &self.overwritten_region {
            return ow_region.to_owned();
        };

        // next, if there's an `using_region` field in the config file, use it.
        if let Some(using_region_name_in_config) =
            &self.config.to_owned().and_then(|x| x.using_region)
        {
            return region_from_str(Some(using_region_name_in_config.to_owned())) // Option<Region>
                .expect("Region name in the config file is invalid.");
        };

        // otherwise, come down to "default region" of your environment.
        // e.g. region set via AWS CLI (check: $ aws configure get region), or environment variable `AWS_DEFAULT_REGION`.
        //      ref: https://docs.rs/rusoto_signature/0.42.0/src/rusoto_signature/region.rs.html#282-290
        //      ref: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html
        Region::default()
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

    pub fn effective_cache_key(&self) -> String {
        return format!(
            "{}/{}",
            &self.effective_region().name(),
            &self.effective_table_name()
        );
    }

    pub fn cached_using_table_schema(&self) -> Option<TableSchema> {
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

    pub fn with_region(mut self, ec2_region: &rusoto_ec2::Region) -> Self {
        self.overwritten_region =
            Some(Region::from_str(&ec2_region.to_owned().region_name.unwrap()).unwrap());
        self
    }

    pub fn with_table(mut self, table: &str) -> Self {
        self.overwritten_table_name = Some(table.to_owned());
        self
    }
}

// FYI: https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/wrap_error.html
#[derive(Debug)]
pub enum DyneinConfigError {
    IO(IOError),
    YAML(SerdeYAMLError),
    HomeDir,
}

impl fmt::Display for DyneinConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DyneinConfigError::IO(ref e) => e.fmt(f),
            DyneinConfigError::YAML(ref e) => e.fmt(f),
            DyneinConfigError::HomeDir => write!(f, "failed to find Home directory"),
        }
    }
}

impl error::Error for DyneinConfigError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            // The cause is the underlying implementation error type. Is implicitly cast to the trait object `&error::Error`.
            // This works because the underlying type already implements the `Error` trait.
            DyneinConfigError::IO(ref e) => Some(e),
            DyneinConfigError::YAML(ref e) => Some(e),
            DyneinConfigError::HomeDir => None,
        }
    }
}

// Implement the conversion from existing error like `serde_yaml::Error` to `DyneinConfigError`.
// This will be automatically called by `?` if underlying errors needs to be converted into a `DyneinConfigError`.
impl From<IOError> for DyneinConfigError {
    fn from(e: IOError) -> DyneinConfigError {
        DyneinConfigError::IO(e)
    }
}
impl From<SerdeYAMLError> for DyneinConfigError {
    fn from(e: SerdeYAMLError) -> DyneinConfigError {
        DyneinConfigError::YAML(e)
    }
}

/* =================================================
Public functions
================================================= */

// Receives given --region option string, including "local", return Region struct.
pub fn region_from_str(s: Option<String>) -> Option<Region> {
    match s.as_deref() {
        Some("local") => Some(region_dynamodb_local(8000)),
        Some(x) => Region::from_str(&x).ok(), // convert Result<T, E> into Option<T>
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
            fs::write(
                &retrieve_dynein_file_path(DyneinFileType::ConfigFile)?,
                yaml_string,
            )?;
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
                "Config file doesn't exist in the path, hence creating a blank file: {}",
                e
            );
            let yaml_string = serde_yaml::to_string(&Cache {
                ..Default::default()
            })
            .unwrap();
            fs::write(
                &retrieve_dynein_file_path(DyneinFileType::CacheFile)?,
                yaml_string,
            )?;
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
        .or_else(|| positional_arg_table_name.as_ref());
    match target_table {
        Some(tbl) => {
            debug!("describing the table: {}", tbl);
            let region = cx.effective_region();
            let tbl = tbl.clone();
            let desc: TableDescription = describe_table_api(&region, tbl.clone()).await;
            save_using_target(cx, desc)?;
            println!("Now you're using the table '{}' ({}).", tbl, &region.name());
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
        &region.name(),
        &table_name
    );

    // retrieve current cache from Context and update target table desc.
    // key to save the table desc is "<RegionName>/<TableName>" -- e.g. "us-west-2/app_data"
    let mut cache: Cache = cx.clone().cache.expect("cx should have cache");
    let cache_key = format!("{}/{}", region.name(), table_name);

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
            region: String::from(region.name()),
            name: table_name,
            pk: typed_key("HASH", &desc).expect("pk should exist"),
            sk: typed_key("RANGE", &desc),
            indexes: index_schemas(&desc),
            mode: control::extract_mode(&desc.billing_mode_summary),
        },
    );
    cache.tables = Some(table_schema_hashmap);

    // write to cache file
    let cache_yaml_string = serde_yaml::to_string(&cache)?;
    debug!(
        "this YAML will be written to the cache file: {:#?}",
        &cache_yaml_string
    );
    fs::write(
        retrieve_dynein_file_path(DyneinFileType::CacheFile)?,
        cache_yaml_string,
    )?;

    Ok(())
}

/// Physicall remove config and cache file.
pub fn remove_dynein_files() -> Result<(), DyneinConfigError> {
    fs::remove_file(retrieve_dynein_file_path(DyneinFileType::ConfigFile)?)?;
    fs::remove_file(retrieve_dynein_file_path(DyneinFileType::CacheFile)?)?;
    Ok(())
}

/// returns Option of a tuple (attribute_name, attribute_type (S/N/B)).
/// Used when you want to know "what is the Partition Key name and its data type of this table".
pub fn typed_key(pk_or_sk: &str, desc: &TableDescription) -> Option<Key> {
    // extracting key schema of "base table" here
    let ks = desc.clone().key_schema.unwrap();
    typed_key_for_schema(pk_or_sk, &ks, &desc.clone().attribute_definitions.unwrap())
}

/// Receives key data type (HASH or RANGE), KeySchemaElement(s), and AttributeDefinition(s),
/// In many cases it's called by typed_key, but when retrieving index schema, this method can be used directly so put it as public.
pub fn typed_key_for_schema(
    pk_or_sk: &str,
    ks: &[KeySchemaElement],
    attrs: &[AttributeDefinition],
) -> Option<Key> {
    // Fetch Partition Key ("HASH") or Sort Key ("RANGE") from given Key Schema. pk should always exists, but sk may not.
    let target_key = ks.iter().find(|x| x.key_type == pk_or_sk);
    target_key.map(|key| Key {
        name: key.clone().attribute_name,
        // kind should be one of S/N/B, Which can be retrieved from AttributeDefinition's attribute_type.
        kind: KeyType::from_str(
            &attrs
                .iter()
                .find(|at| at.attribute_name == key.attribute_name)
                .expect("primary key should be in AttributeDefinition.")
                .attribute_type,
        )
        .unwrap(),
    })
}

// If you explicitly specify target table by `--table/-t` option, this function executes DescribeTable API to gather table schema info.
// Otherwise, load table schema info from config file.
// fn table_schema(region: &Region, config: &config::Config, table_overwritten: Option<String>) -> TableSchema {
pub async fn table_schema(cx: &Context) -> TableSchema {
    match cx.overwritten_table_name.to_owned() {
        // It's possible that users pass --table without calling `dy use` for any table. Thus collect all data from DescribeTable results.
        Some(table_name) => {
            // TODO: reduce # of DescribeTable API calls. table_schema function is called every time you do something.
            let desc: TableDescription = describe_table_api(
                &cx.effective_region(),
                table_name, /* should be equal to 'cx.effective_table_name()' */
            )
            .await;

            TableSchema {
                region: String::from(cx.effective_region().name()),
                name: desc.clone().table_name.unwrap(),
                pk: typed_key("HASH", &desc).expect("pk should exist"),
                sk: typed_key("RANGE", &desc),
                indexes: index_schemas(&desc),
                mode: control::extract_mode(&desc.billing_mode_summary),
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
                kind: IndexType::GSI,
                pk: typed_key_for_schema("HASH", &gsi.key_schema.clone().unwrap(), attr_defs)
                    .expect("pk should exist"),
                sk: typed_key_for_schema("RANGE", &gsi.key_schema.unwrap(), attr_defs),
            });
        }
    };

    if let Some(lsis) = desc.clone().local_secondary_indexes {
        for lsi in lsis {
            indexes.push(IndexSchema {
                name: lsi.index_name.unwrap(),
                kind: IndexType::LSI,
                pk: typed_key_for_schema("HASH", &lsi.key_schema.clone().unwrap(), attr_defs)
                    .expect("pk should exist"),
                sk: typed_key_for_schema("RANGE", &lsi.key_schema.unwrap(), attr_defs),
            });
        }
    };

    if indexes.is_empty() {
        None
    } else {
        Some(indexes)
    }
}

/// Originally intended to be called by describe_table function, which is called from `$ dy desc`,
/// however it turned out that DescribeTable API result is useful in various logic, separated API into this standalone function.
pub async fn describe_table_api(region: &Region, table_name: String) -> TableDescription {
    let ddb = DynamoDbClient::new(region.clone());
    let req: DescribeTableInput = DescribeTableInput { table_name };

    match ddb.describe_table(req).await {
        Err(e) => {
            debug!("DescribeTable API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        }
        Ok(res) => {
            let desc: TableDescription = res.table.expect("This message should not be shown.");
            debug!("Received DescribeTable Result: {:?}\n", desc);
            desc
        }
    }
}

pub fn bye(code: i32, msg: &str) {
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
    Region::Custom {
        name: "local".to_owned(),
        endpoint: endpoint_url,
    }
}

fn retrieve_dynein_file_path(dft: DyneinFileType) -> Result<String, DyneinConfigError> {
    let filename = match dft {
        DyneinFileType::ConfigFile => CONFIG_FILE_NAME,
        DyneinFileType::CacheFile => CACHE_FILE_NAME,
    };

    Ok(format!("{}/{}", retrieve_or_create_dynein_dir()?, filename))
}

fn retrieve_or_create_dynein_dir() -> Result<String, DyneinConfigError> {
    match dirs::home_dir() {
        None => Err(DyneinConfigError::HomeDir),
        Some(home) => {
            let dir = format!("{}/{}", home.to_str().unwrap(), CONFIG_DIR);
            if !path::Path::new(&dir).exists() {
                debug!("Creating dynein config directory: {}", dir);
                fs::create_dir(&dir)?;
            };
            Ok(dir)
        }
    }
}

/// This function updates `using_region` and `using_table` in config.yml,
/// and at the same time inserts TableDescription of the target table into cache.yml.
fn save_using_target(cx: &mut Context, desc: TableDescription) -> Result<(), DyneinConfigError> {
    let table_name: String = desc
        .table_name
        .clone()
        .expect("desc should have table name");

    // retrieve current config from Context and update "using target".
    let region = Some(String::from(cx.effective_region().name()));
    let mut config = cx.config.as_mut().expect("cx should have config");
    config.using_region = region;
    config.using_table = Some(table_name);
    debug!("config file will be updated with: {:?}", config);

    // write to config file
    let config_yaml_string = serde_yaml::to_string(config)?;
    fs::write(
        retrieve_dynein_file_path(DyneinFileType::ConfigFile)?,
        config_yaml_string,
    )?;

    // save target table info into cache.
    insert_to_table_cache(&cx, desc)?;

    Ok(())
}

/* =================================================
Unit Tests
================================================= */

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::str::FromStr; // to utilize Region::from_str // for unit tests

    #[test]
    fn test_context_functions() -> Result<(), Box<dyn Error>> {
        let cx1 = Context {
            region: None,
            config: None,
            cache: None,
            overwritten_region: None,
            overwritten_table_name: None,
            output: None,
        };
        assert_eq!(cx1.effective_region(), Region::default());
        // cx1.effective_table_name(); ... exit(1)

        let cx2 = Context {
            region: None,
            config: Some(Config {
                using_region: Some(String::from("ap-northeast-1")),
                using_table: Some(String::from("cfgtbl")),
            }),
            cache: None,
            overwritten_region: None,
            overwritten_table_name: None,
            output: None,
        };
        assert_eq!(cx2.effective_region(), Region::from_str("ap-northeast-1")?);
        assert_eq!(cx2.effective_table_name(), String::from("cfgtbl"));

        let mut cx3 = cx2.clone();
        cx3.overwritten_region = Some(Region::from_str("us-east-1")?); // --region us-east-1
        cx3.overwritten_table_name = Some(String::from("argtbl")); // --table argtbl
        assert_eq!(cx3.effective_region(), Region::from_str("us-east-1")?);
        assert_eq!(cx3.effective_table_name(), String::from("argtbl"));

        let mut cx4 = cx2.clone();
        cx4.overwritten_region = Some(Region::from_str("us-east-1")?); // --region us-east-1
        assert_eq!(cx4.effective_region(), Region::from_str("us-east-1")?);
        assert_eq!(cx4.effective_table_name(), String::from("cfgtbl"));

        let mut cx5 = cx2.clone();
        cx5.overwritten_table_name = Some(String::from("argtbl")); // --table argtbl
        assert_eq!(cx5.effective_region(), Region::from_str("ap-northeast-1")?);
        assert_eq!(cx5.effective_table_name(), String::from("argtbl"));

        Ok(())
    }
}
