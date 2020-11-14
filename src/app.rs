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

use ::serde::{Serialize, Deserialize};
use dirs;
use log::{debug,error,info};
use rusoto_core::Region;
use rusoto_dynamodb::*;
use serde_yaml::Error as SerdeYAMLError;
use std::error;
use std::fmt::{self, Display, Formatter, Error as FmtError};
use std::fs;
use std::io::Error as IOError;
use std::path;
use std::str::FromStr;

use super::control;

/* =================================================
   struct / enum / const
   ================================================= */

const CONFIG_DIR: &'static str = ".dynein";
const CONFIG_FILE_NAME: &'static str = "config.yml";


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
    pub fn display(&self) -> String { format!("{} ({})", self.name, self.kind) }
}


/// Restrict acceptable DynamoDB data types for primary keys.
///     enum witn methods/FromStr ref: https://docs.rs/rusoto_signature/0.42.0/src/rusoto_signature/region.rs.html#226-258
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum KeyType { S, N, B, }

/// implement Display for KeyType to simply print a single letter "S", "N", or "B".
impl Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            KeyType::S => "S",
            KeyType::N => "N",
            KeyType::B => "B"
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseKeyTypeError { message: String, }

impl std::error::Error for ParseKeyTypeError { fn description(&self) -> &str { &self.message } }

impl Display for ParseKeyTypeError { fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> { write!(f, "{}", self.message) } }

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
pub enum IndexType { GSI, LSI, }


pub enum Messages {
    NoEffectiveTable,
}

impl fmt::Display for Messages {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Messages::NoEffectiveTable => "
To execute commands you must specify target table one of following ways:
    * $ dy --region <region> use <your_table> ... save target region and table. After that you don't need to pass region/table [RECOMMENDED].
    * $ dy --region <region> --table <your_table> scan ... data operations like 'scan', use --region and --table options.
    * $ dy --region <region> desc <your_table> ... to describe a table, you have to specify table name.
To list all tables in all regions, try:
    * $ dy ls --all-regions",
        })
    }
}


/*
* This is a struct for dynein configuration.
* Currently the only information in the config file is the table information that being used,
* but in future it's possible that we separate cached table information from configurations,
* which may contain "expiration time for cached table", "default region", and "default table name" etc.
*/
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub table: Option<TableSchema>,
}


#[derive(Debug, Clone)]
pub struct Context {
    pub region: Option<Region>,
    pub config: Option<Config>,
    pub overwritten_region: Option<Region>, // --region option
    pub overwritten_table_name: Option<String>,  // --table option
    pub output: Option<String>,
}


/*
 When region/table info is given by command line arguments (--region/--table),
 Context object has overwritten_region/overwritten_table_name values. Implemented in main.rs.
 Overwritten information is retrieved with `effective_*` functions as 1st priority.

 For Data Plane commands (put, scan, query, etc) and table-specific commands (desc, use): --region and --table should be "all-or-nothing".
 For Control Plane coommands (table list, table create, etc): passing only --region is acceptable.
*/
impl Context {
    pub fn effective_region(&self) -> Region {
        // if region is overwritten by --region comamnd, use it.
        if let Some(ow_region) = &self.overwritten_region { return ow_region.to_owned(); };

        // next, if there's a region name in currently using table, use it.
        if let Some(table) = self.config.as_ref().and_then(|x| x.clone().table ) { return region_from_str(Some(table.region)).unwrap(); };

        // otherwise, come down to "default region" of your environment.
        // e.g. region set via AWS CLI (check: $ aws configure get region), or environment variable `AWS_DEFAULT_REGION`.
        //      ref: https://docs.rs/rusoto_signature/0.42.0/src/rusoto_signature/region.rs.html#282-290
        //      ref: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html
        return Region::default();
    }

    pub fn effective_table_name(&self) -> String {
        // if table is overwritten by --table command, use it.
        if let Some(ow_table_name) = &self.overwritten_table_name { return ow_table_name.to_owned(); };
        // otherwise, retrieve table name from config file.
        return match self.config.as_ref().and_then(|x| x.table.to_owned() ) {
            Some(table) => table.name,
            // if both of data sources above are not available, raise error and exit the command.
            None => { error!("{}", Messages::NoEffectiveTable); std::process::exit(1); },
        }
    }

    pub fn with_region(mut self, ec2_region: &rusoto_ec2::Region) -> Self {
        self.overwritten_region = Some(Region::from_str(&ec2_region.to_owned().region_name.unwrap()).unwrap());
        return self;
    }

    pub fn with_table(mut self, table: &String) -> Self {
        self.overwritten_table_name = Some(table.to_owned());
        return self;
    }
}


// FYI: https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/wrap_error.html
#[derive(Debug)]
pub enum DyneinConfigError {
    IO(IOError),
    YAML(SerdeYAMLError),
    HomeDir
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
impl From<IOError> for DyneinConfigError { fn from(e: IOError) -> DyneinConfigError { DyneinConfigError::IO(e) } }
impl From<SerdeYAMLError> for DyneinConfigError { fn from(e: SerdeYAMLError) -> DyneinConfigError { DyneinConfigError::YAML(e) } }


/* =================================================
   Public functions
   ================================================= */

// Receives given --region option string, including "local", return Region struct.
pub fn region_from_str(s: Option<String>) -> Option<Region> {
    match s.as_ref().map(|x| x.as_str()) {
        Some("local") => Some(region_dynamodb_local(8000)),
        Some(x) => Region::from_str(&x).ok(), // convert Result<T, E> into Option<T>
        None => None,
    }
}


fn region_dynamodb_local(port: u32) -> Region {
    let endpoint_url = format!("http://localhost:{}", port);
    debug!("setting DynamoDB Local '{}' as target region.", &endpoint_url);
    return Region::Custom {
        name: "local".to_owned(),
        endpoint: endpoint_url.to_owned(),
    };
}

/// Loads dynein config file (YAML format) and return config struct as a result.
/// If it cannot find config file, create blank config.
pub fn load_or_touch_config_file(first_try: bool) -> Result<Config, DyneinConfigError> {
    let path = retrieve_config_file_path();
    debug!("Loading Config File: {}", path);

    match fs::read_to_string(&path) {
        Ok(_str) => {
            let config: Config = serde_yaml::from_str(&_str)?;
            debug!("Loaded current config: {:?}", config);
            Ok(config)
        },
        Err(e) => {
            if !first_try { return Err(DyneinConfigError::from(e)) };
            info!("Config file doesn't exist in the path, hence creating a blank file: {}", e);
            touch_config_file()?;
            load_or_touch_config_file(false) // set fisrt_try flag to false in order to avoid infinite loop.
        }
    }
}


pub async fn use_table(cx: &Context) {
    if let Some(tbl) = &cx.overwritten_table_name {
        debug!("describing the table: {}", &tbl);
        let region = cx.effective_region();
        let desc: TableDescription = describe_table_api(&region, tbl.clone()).await;
        let config = cx.config.clone().unwrap();
        save_table(region, config, desc);
    } else {
        bye(1, "ERROR: You have to specify a tabel to use by --table/-t option.");
    }
}


/// Physicall remove config file.
pub fn remove_config_file() -> std::io::Result<()> {
    fs::remove_file(retrieve_config_file_path())?;
    Ok(())
}


/// Create config file with blank (None) for each field.
pub fn touch_config_file() -> std::io::Result<()> {
    let yaml_string = serde_yaml::to_string(&Config { table: None }).unwrap();
    fs::write(&retrieve_config_file_path(), yaml_string).expect("Could not write to file!");
    Ok(())
}


/// returns Option of a tuple (attribute_name, attribute_type (S/N/B)).
/// Used when you want to know "what is the Partition Key name and its data type of this table".
pub fn typed_key(pk_or_sk: &str, desc: &TableDescription) -> Option<Key> {
    // extracting key schema of "base table" here
    let ks = desc.clone().key_schema.unwrap();
    return typed_key_for_schema(pk_or_sk, &ks, &desc.clone().attribute_definitions.unwrap());
}


/// Receives key data type (HASH or RANGE), KeySchemaElement(s), and AttributeDefinition(s),
/// In many cases it's called by typed_key, but when retrieving index schema, this method can be used directly so put it as public.
pub fn typed_key_for_schema(pk_or_sk: &str, ks: &Vec<KeySchemaElement>, attrs: &Vec<AttributeDefinition>) -> Option<Key> {
    // Fetch Partition Key ("HASH") or Sort Key ("RANGE") from given Key Schema. pk should always exists, but sk may not.
    let target_key = ks.iter().find(|x| x.key_type == pk_or_sk);
    return target_key.map(|key|
        Key {
            name: key.clone().attribute_name,
            // kind should be one of S/N/B, Which can be retrieved from AttributeDefinition's attribute_type.
            kind: KeyType::from_str(
                      &attrs.iter().find(|at| at.attribute_name == key.attribute_name)
                                   .expect("primary key should be in AttributeDefinition.").attribute_type
                  ).unwrap(),
        }
    );
}


// If you explicitly specify target table by `--table/-t` option, this function executes DescribeTable API to gather table schema info.
// Otherwise, load table schema info from config file.
// fn table_schema(region: &Region, config: &config::Config, table_overwritten: Option<String>) -> TableSchema {
pub async fn table_schema(cx: &Context) -> TableSchema {
    match cx.overwritten_table_name.to_owned() {
        // It's possible that users pass --table without calling `dy use` for any table. Thus collect all data from DescribeTable results.
        Some(table_name) => {
            // TODO: reduce # of DescribeTable API calls. table_schema function is called every time you do something.
            let desc: TableDescription = describe_table_api(&cx.effective_region(), table_name /* should be equal to 'cx.effective_table_name()' */).await;

            return TableSchema {
                region: String::from(cx.effective_region().name()),
                name: desc.clone().table_name.clone().unwrap().to_string(),
                pk: typed_key("HASH",  &desc).expect("pk should exist"),
                sk: typed_key("RANGE", &desc),
                indexes: index_schemas(&desc),
                mode: control::extract_mode(&desc.billing_mode_summary),
            }
        },
        None => { // simply maps config data into TableSchema struct.
            debug!("current context {:#?}", cx);
            return cx.config.clone().unwrap().table.unwrap_or_else(|| {
                error!("{}", Messages::NoEffectiveTable); std::process::exit(1)
            });
        },
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
                pk: typed_key_for_schema("HASH", &gsi.key_schema.clone().unwrap(), attr_defs).expect("pk should exist"),
                sk: typed_key_for_schema("RANGE", &gsi.key_schema.unwrap(), attr_defs),
            });
        };
    };

    if let Some(lsis) = desc.clone().local_secondary_indexes {
        for lsi in lsis {
            indexes.push(IndexSchema {
                name: lsi.index_name.unwrap(),
                kind: IndexType::LSI,
                pk: typed_key_for_schema("HASH", &lsi.key_schema.clone().unwrap(), attr_defs).expect("pk should exist"),
                sk: typed_key_for_schema("RANGE", &lsi.key_schema.unwrap(), attr_defs),
            });
        };
    };

    return if indexes.len() == 0 { None } else { Some(indexes) };
}


/// Originally intended to be called by describe_table function, which is called from `$ dy desc`,
/// however it turned out that DescribeTable API result is useful in various logic, separated API into this standalone function.
pub async fn describe_table_api(region: &Region, table_name: String) -> TableDescription {
    let ddb = DynamoDbClient::new(region.clone());
    let req: DescribeTableInput = DescribeTableInput { table_name: table_name };

    match ddb.describe_table(req).await {
        Err(e) => {
            debug!("DescribeTable API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        },
        Ok(res) => {
            let desc: TableDescription = res.table.expect("This message should not be shown.");
            debug!("Received DescribeTable Result: {:?}\n", desc);
            return desc;
        }
    }
}


pub fn bye(code: i32, msg: &str) {
    println!("{}", msg);
    std::process::exit(code);
}


pub fn save_table(region: Region, config: Config, desc: TableDescription) {
    let path = retrieve_config_file_path();

    let mut new_config = config.clone();
    new_config.table = None; // reset existing table info
    new_config.table = Some(TableSchema {
        region: String::from(region.name()),
        name: desc.table_name.clone().unwrap(),
        pk: typed_key("HASH", &desc).expect("pk should exist"),
        sk: typed_key("RANGE", &desc),
        indexes: index_schemas(&desc),
        mode: control::extract_mode(&desc.billing_mode_summary),
    });

    let yaml_string = serde_yaml::to_string(&new_config).expect("Failed to execute serde_yaml::to_string");
    fs::write(path, yaml_string).expect("Could not write to file!");
    debug!("Config Updated: {:#?}", &new_config);
}


/* =================================================
   Private functions
   ================================================= */

fn retrieve_config_file_path () -> String { format!("{}/{}", retrieve_config_dir().unwrap(), CONFIG_FILE_NAME) }
fn retrieve_config_dir() -> Result<String, DyneinConfigError> {
    return match dirs::home_dir() {
        None => Err(DyneinConfigError::HomeDir),
        Some(home) => {
            let dir = format!("{}/{}", home.to_str().unwrap(), CONFIG_DIR);
            if !path::Path::new(&dir).exists() {
                debug!("Creating dynein config directory: {}", dir);
                fs::create_dir(&dir)?;
            };
            Ok(dir)
        },
    }
}


/* =================================================
   Unit Tests
   ================================================= */

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::str::FromStr; // to utilize Region::from_str
    use super::*; // for unit tests

    #[test]
    fn test_context_functions() -> Result<(), Box<dyn Error>> {
        let cx1 = Context {
            region: None,
            config: None,
            overwritten_region: None,
            overwritten_table_name: None,
            output: None,
        };
        assert_eq!(cx1.effective_region(), Region::default());
        // cx1.effective_table_name(); ... exit(1)

        let cx2 = Context {
            region: None,
            config: Some(Config {
                table: Some(TableSchema {
                    region: String::from("ap-northeast-1"),
                    name: String::from("cfgtbl"),
                    pk: Key { name: String::from("pk"), kind: KeyType::S },
                    sk: None,
                    indexes: None,
                    mode: control::Mode::OnDemand,
                }),
            }),
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
