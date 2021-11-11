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

// This module interact with DynamoDB Control Plane APIs
use ::serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::future::join_all;
use log::{debug, error};
use rusoto_core::Region;
use rusoto_dynamodb::*;
use rusoto_ec2::{DescribeRegionsRequest, Ec2, Ec2Client};
use std::{
    io::{self, Error as IOError, Write},
    time,
};

use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use tabwriter::TabWriter;

use super::app;

/* =================================================
struct / enum / const
================================================= */

// TableDescription doesn't implement Serialize
// https://docs.rs/rusoto_dynamodb/0.42.0/rusoto_dynamodb/struct.TableDescription.html
#[derive(Serialize, Deserialize, Debug)]
struct PrintDescribeTable {
    name: String,
    region: String,
    status: String,
    schema: PrintPrimaryKeys,

    mode: Mode,
    capacity: Option<PrintCapacityUnits>,

    gsi: Option<Vec<PrintSecondaryIndex>>,
    lsi: Option<Vec<PrintSecondaryIndex>>,

    stream: Option<String>,

    count: i64,
    size_bytes: i64,
    created_at: String,
}

const PROVISIONED_API_SPEC: &str = "PROVISIONED";
const ONDEMAND_API_SPEC: &str = "PAY_PER_REQUEST";

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Mode {
    Provisioned,
    OnDemand,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrintPrimaryKeys {
    pk: String,
    sk: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrintCapacityUnits {
    wcu: i64,
    rcu: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrintSecondaryIndex {
    name: String,
    schema: PrintPrimaryKeys,
    capacity: Option<PrintCapacityUnits>,
}

/* =================================================
Public functions
================================================= */

pub async fn list_tables_all_regions(cx: app::Context) {
    let ec2 = Ec2Client::new(cx.effective_region());
    let input: DescribeRegionsRequest = DescribeRegionsRequest {
        ..Default::default()
    };
    match ec2.describe_regions(input).await {
        Err(e) => {
            error!("{}", e.to_string());
            std::process::exit(1);
        }
        Ok(res) => {
            join_all(
                res.regions
                    .expect("regions should exist") // Vec<Region>
                    .iter()
                    .map(|r| list_tables(cx.clone().with_region(r))),
            )
            .await;
        }
    };
}

pub async fn list_tables(cx: app::Context) {
    let table_names = list_tables_api(cx.clone()).await;

    println!(
        "DynamoDB tables in region: {}",
        cx.effective_region().name()
    );
    if table_names.is_empty() {
        return println!("  No table in this region.");
    }

    // if let Some(table_in_config) = cx.clone().config.and_then(|x| x.table) {
    if let Some(table_in_config) = cx.clone().cached_using_table_schema() {
        for table_name in table_names {
            if cx.clone().effective_region().name() == table_in_config.region
                && table_name == table_in_config.name
            {
                println!("* {}", table_name);
            } else {
                println!("  {}", table_name);
            }
        }
    } else {
        debug!("No table information (currently using table) is found on config file");
        for table_name in table_names {
            println!("  {}", table_name)
        }
    }
}

/// Executed when you call `$ dy desc --all-tables`.
/// Note that `describe_table` function calls are executed in parallel (async + join_all).
pub async fn describe_all_tables(cx: app::Context) {
    let table_names = list_tables_api(cx.clone()).await;
    join_all(
        table_names
            .into_iter()
            .map(|t| describe_table(cx.clone(), Some(t))),
    )
    .await;
}

/// Executed when you call `$ dy desc (table)`. Retrieve TableDescription via describe_table_api function,
/// then print them in convenient way using print_table_description function (default/yaml).
pub async fn describe_table(cx: app::Context, target_table_to_desc: Option<String>) {
    debug!("context: {:#?}", &cx);
    debug!("positional arg table name: {:?}", &target_table_to_desc);
    let new_context = if let Some(t) = target_table_to_desc {
        cx.with_table(t.as_str())
    } else {
        cx
    };

    let desc: TableDescription = app::describe_table_api(
        &new_context.effective_region(),
        new_context.effective_table_name(),
    )
    .await;
    debug!(
        "Retrieved table to describe is: '{}' table in '{}' region.",
        &new_context.effective_table_name(),
        &new_context.effective_region().name()
    );

    // save described table info into cache for future use.
    // Note that when this functiono is called from describe_all_tables, not all tables would be cached as calls are parallel.
    match app::insert_to_table_cache(&new_context, desc.clone()) {
        Ok(_) => debug!("Described table schema was written to the cache file."),
        Err(e) => println!(
            "Failed to write table schema to the cache with follwoing error: {:?}",
            e
        ),
    };

    match new_context.clone().output.as_deref() {
        None | Some("yaml") => print_table_description(new_context.effective_region(), desc),
        // Some("raw") => println!("{:#?}", desc),
        Some(_) => {
            println!("ERROR: unsupported output type.");
            std::process::exit(1);
        }
    }
}

/// Receives region (just to show in one line for reference) and TableDescription,
/// print them in readable YAML format. NOTE: '~' representes 'null' or 'no value' in YAML syntax.
pub fn print_table_description(region: Region, desc: TableDescription) {
    let attr_defs = desc.clone().attribute_definitions.unwrap();
    let mode = extract_mode(&desc.billing_mode_summary);

    let print_table: PrintDescribeTable = PrintDescribeTable {
        name: String::from(&desc.clone().table_name.unwrap()),
        region: String::from(region.name()),
        status: String::from(&desc.clone().table_status.unwrap()),
        schema: PrintPrimaryKeys {
            pk: app::typed_key("HASH", &desc)
                .expect("pk should exist")
                .display(),
            sk: app::typed_key("RANGE", &desc).map(|k| k.display()),
        },

        mode: mode.clone(),
        capacity: extract_capacity(&mode, &desc.provisioned_throughput),

        gsi: extract_secondary_indexes(&mode, &attr_defs, desc.global_secondary_indexes),
        lsi: extract_secondary_indexes(&mode, &attr_defs, desc.local_secondary_indexes),
        stream: extract_stream(desc.latest_stream_arn, desc.stream_specification),

        size_bytes: desc.table_size_bytes.unwrap(),
        count: desc.item_count.unwrap(),
        created_at: epoch_to_rfc3339(desc.creation_date_time.unwrap()),
    };
    println!("{}", serde_yaml::to_string(&print_table).unwrap());
}

/// This function is designed to be called from dynein command, mapped in main.rs.
/// Note that it simply ignores --table option if specified. Newly created table name should be given by the 1st argument "name".
pub async fn create_table(cx: app::Context, name: String, given_keys: Vec<String>) {
    if given_keys.is_empty() || given_keys.len() >= 3 {
        error!("You should pass one or two key definitions with --keys option");
        std::process::exit(1);
    };

    match create_table_api(cx.clone(), name, given_keys).await {
        Ok(desc) => print_table_description(cx.effective_region(), desc),
        Err(e) => {
            debug!("CreateTable API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        }
    }
}

pub async fn create_table_api(
    cx: app::Context,
    name: String,
    given_keys: Vec<String>,
) -> Result<TableDescription, rusoto_core::RusotoError<rusoto_dynamodb::CreateTableError>> {
    debug!(
        "Trying to create a table '{}' with keys '{:?}'",
        &name, &given_keys
    );

    let (key_schema, attribute_definitions) = generate_essential_key_definitions(&given_keys);

    let ddb = DynamoDbClient::new(cx.effective_region());
    let req: CreateTableInput = CreateTableInput {
        table_name: name,
        billing_mode: Some(String::from(ONDEMAND_API_SPEC)),
        key_schema,            // Vec<KeySchemaElement>
        attribute_definitions, // Vec<AttributeDefinition>
        ..Default::default()
    };

    ddb.create_table(req).await.map(|res| {
        res.table_description
            .expect("Table Description returned from API should be valid.")
    })
}

pub async fn create_index(cx: app::Context, index_name: String, given_keys: Vec<String>) {
    if given_keys.is_empty() || given_keys.len() >= 3 {
        error!("You should pass one or two key definitions with --keys option");
        std::process::exit(1);
    };
    debug!(
        "Trying to create an index '{}' with keys '{:?}', on table '{}' ",
        &index_name,
        &given_keys,
        &cx.effective_table_name()
    );

    let (key_schema, attribute_definitions) = generate_essential_key_definitions(&given_keys);

    let ddb = DynamoDbClient::new(cx.effective_region());
    let create_gsi_action = CreateGlobalSecondaryIndexAction {
        index_name,
        key_schema,
        projection: Projection {
            projection_type: Some(String::from("ALL")),
            non_key_attributes: None,
        },
        provisioned_throughput: None, // TODO: assign default rcu/wcu if base table is Provisioned mode. currently it works only for OnDemand talbe.
    };
    let gsi_update = GlobalSecondaryIndexUpdate {
        create: Some(create_gsi_action),
        update: None,
        delete: None,
    };
    let req: UpdateTableInput = UpdateTableInput {
        table_name: cx.effective_table_name(),
        attribute_definitions: Some(attribute_definitions), // contains minimum necessary/missing attributes to add to define new GSI.
        global_secondary_index_updates: Some(vec![gsi_update]),
        ..Default::default()
    };

    match ddb.update_table(req).await {
        Err(e) => {
            debug!("UpdateTable API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        }
        Ok(res) => {
            debug!("Returned result: {:#?}", res);
            print_table_description(cx.effective_region(), res.table_description.unwrap());
        }
    }
}

pub async fn update_table(
    cx: app::Context,
    table_name_to_update: String,
    mode_string: Option<String>,
    wcu: Option<i64>,
    rcu: Option<i64>,
) {
    // Retrieve TableDescription of the table to update, current (before update) status.
    let desc: TableDescription =
        app::describe_table_api(&cx.effective_region(), table_name_to_update.clone()).await;

    // Map given string into "Mode" enum. Note that in cmd.rs structopt already limits acceptable values.
    let switching_to_mode: Option<Mode> = match mode_string {
        None => None,
        Some(ms) => match ms.as_str() {
            "provisioned" => Some(Mode::Provisioned),
            "ondemand"    => Some(Mode::OnDemand),
            _ => panic!("You shouldn't see this message as --mode can takes only 'provisioned' or 'ondemand'."),
        },
    };

    // Configure ProvisionedThroughput struct based on argumsnts (mode/wcu/rcu).
    let provisioned_throughput: Option<ProvisionedThroughput> = match &switching_to_mode {
        // when --mode is not given, no mode switch happens. Check the table's current mode.
        None => {
            match extract_mode(&desc.clone().billing_mode_summary) {
                // When currently OnDemand mode and you're not going to change the it, set None for CU.
                Mode::OnDemand => {
                    if wcu.is_some() || rcu.is_some() {
                        println!("Ignoring --rcu/--wcu options as the table mode is OnDemand.");
                    };
                    None
                }
                // When currently Provisioned mode and you're not going to change the it,
                // pass given rcu/wcu, and use current values if missing. Provisioned table should have valid capacity units so unwrap() here.
                Mode::Provisioned => Some(ProvisionedThroughput {
                    read_capacity_units: rcu.unwrap_or_else(|| {
                        desc.clone()
                            .provisioned_throughput
                            .unwrap()
                            .read_capacity_units
                            .unwrap()
                    }),
                    write_capacity_units: wcu.unwrap_or_else(|| {
                        desc.clone()
                            .provisioned_throughput
                            .unwrap()
                            .write_capacity_units
                            .unwrap()
                    }),
                }),
            }
        }
        // When the user trying to switch mode.
        Some(target_mode) => match target_mode {
            // when switching Provisioned->OnDemand mode, ProvisionedThroughput can be None.
            Mode::OnDemand => {
                if wcu.is_some() || rcu.is_some() {
                    println!("Ignoring --rcu/--wcu options as --mode ondemand.");
                };
                None
            }
            // when switching OnDemand->Provisioned mode, set given wcu/rcu, fill with "5" as a default if not given.
            Mode::Provisioned => Some(ProvisionedThroughput {
                read_capacity_units: rcu.unwrap_or(5),
                write_capacity_units: wcu.unwrap_or(5),
            }),
        },
    };

    // TODO: support updating CU of the table with GSI. If the table has GSIs, you must specify CU for them at the same time.
    // error message: One or more parameter values were invalid: ProvisionedThroughput must be specified for index: xyz_index,abc_index2
    //   if table has gsi
    //     build GlobalSecondaryIndexUpdates { [... current values ...] }

    match update_table_api(
        cx.clone(),
        table_name_to_update,
        switching_to_mode,
        provisioned_throughput,
    )
    .await
    {
        Ok(desc) => print_table_description(cx.effective_region(), desc),
        Err(e) => {
            debug!("UpdateTable API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        }
    }
}

/// UpdateTable API accepts following parameters (ref: https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateTable.html):
///   * [x] TableName (required)
///   * [x] BillingMode
///   * [x] ProvisionedThroughput > obj
///   * [-] AttributeDefinitions > array of AttributeDefinition obj
///   * [-] GlobalSecondaryIndexUpdates > Create/Update/Delete and details of the update on GSIs
///   * [-] ReplicaUpdates > Create/Update/Delete and details of the update on Global Tbles replicas
///   * [] SSESpecification > obj
///   * [] StreamSpecification > obj
/// [+] = supported, [-] = implemented (or plan to so) in another location, [] = not yet supported
/// Especially note that you should explicitly pass GSI update parameter to make any change on GSI.
async fn update_table_api(
    cx: app::Context,
    table_name_to_update: String,
    switching_to_mode: Option<Mode>,
    provisioned_throughput: Option<ProvisionedThroughput>,
) -> Result<TableDescription, rusoto_core::RusotoError<rusoto_dynamodb::UpdateTableError>> {
    debug!("Trying to update the table '{}'.", &table_name_to_update);

    let ddb = DynamoDbClient::new(cx.effective_region());

    let req: UpdateTableInput = UpdateTableInput {
        table_name: table_name_to_update,
        billing_mode: switching_to_mode.map(mode_to_billing_mode_api_spec),
        provisioned_throughput,
        // NOTE: In this function we set `global_secondary_index_updates` to None. GSI update is handled in different commands (e.g. dy admin create index xxx --keys)
        global_secondary_index_updates: None, /* intentional */
        ..Default::default()
    };

    ddb.update_table(req).await.map(|res| {
        res.table_description
            .expect("Table Description returned from API should be valid.")
    })
}

pub async fn delete_table(cx: app::Context, name: String, skip_confirmation: bool) {
    debug!("Trying to delete a table '{}'", &name);

    let msg = format!("You're trying to delete a table '{}'. Are you OK?", &name);
    if !skip_confirmation && !Confirm::new().with_prompt(&msg).interact().unwrap() {
        println!("The table delete operation has been canceled.");
        return;
    }

    let ddb = DynamoDbClient::new(cx.effective_region());

    // The only argument can be passed to DeleteTable operation is "table_name".
    // https://rusoto.github.io/rusoto/rusoto_dynamodb/struct.DeleteTableInput.html
    let req: DeleteTableInput = DeleteTableInput { table_name: name };

    match ddb.delete_table(req).await {
        Err(e) => {
            debug!("DeleteTable API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        }
        Ok(res) => {
            debug!("Returned result: {:#?}", res);
            println!(
                "Delete operation for the table '{}' has been started.",
                res.table_description.unwrap().table_name.unwrap()
            );
        }
    }
}

/// Takes on-demand Backup for the table. It takes --all-tables option but it doesn't take any effect.
///
/// OnDemand backup is a type of backups that can be manually created. Another type is called PITR (Point-In-Time-Restore) but dynein doesn't support it for now.
/// For more information about DynamoDB on-demand backup: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html
pub async fn backup(cx: app::Context, all_tables: bool) {
    // this "backup" function is called only when --list is NOT given. So, --all-tables would be ignored.
    if all_tables {
        println!("NOTE: --all-tables option is ignored without --list option. Just trying to create a backup for the target table...")
    };
    debug!(
        "Taking a backof of the table '{}'",
        cx.effective_table_name()
    );
    let epoch: u64 = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .expect("should be able to generate UNIX EPOCH")
        .as_secs();

    let ddb = DynamoDbClient::new(cx.effective_region());

    // You need to pass "table_name" and "backup_name". There's no other fields.
    // https://rusoto.github.io/rusoto/rusoto_dynamodb/struct.CreateBackupInput.html
    let req: CreateBackupInput = CreateBackupInput {
        table_name: cx.effective_table_name(),
        backup_name: format!("{}--dynein-{}", cx.effective_table_name(), epoch),
    };

    debug!("this is the req: {:?}", req);

    match ddb.create_backup(req).await {
        Err(e) => {
            debug!("CreateBackup API call got an error -- {:#?}", e);
            app::bye(1, &e.to_string());
        }
        Ok(res) => {
            debug!("Returned result: {:#?}", res);
            let details = res.backup_details.expect("should have some details");
            println!("Backup creation has been started:");
            println!(
                "  Backup Name: {} (status: {})",
                details.backup_name, details.backup_status
            );
            println!("  Backup ARN: {}", details.backup_arn);
            println!(
                "  Backup Size: {} bytes",
                details.backup_size_bytes.expect("should have table size")
            );
        }
    }
}

/// List backups for a specified table. With --all-tables option all backups for all tables in the region are shown.
pub async fn list_backups(cx: app::Context, all_tables: bool) -> Result<(), IOError> {
    let backups = list_backups_api(&cx, all_tables).await;
    let mut tw = TabWriter::new(io::stdout());
    // First defining header
    tw.write_all(
        ((vec!["Table", "Status", "CreatedAt", "BackupName (size)"].join("\t")) + "\n").as_bytes(),
    )?;
    for backup in backups {
        let line = vec![
            backup.table_name.expect("table name should exist"),
            backup.backup_status.expect("status should exist"),
            epoch_to_rfc3339(
                backup
                    .backup_creation_date_time
                    .expect("creation date should exist"),
            ),
            backup.backup_name.expect("backup name should exist")
                + &format!(
                    " ({} bytes)",
                    backup.backup_size_bytes.expect("size should exist")
                ),
            String::from("\n"),
        ];
        tw.write_all(line.join("\t").as_bytes())?;
    }
    tw.flush()?;
    Ok(())
}

/// This function restores DynamoDB table from specified backup data.
/// If you don't specify backup data (name) explicitly, dynein will list backups and you can select out of them.
/// Currently overwriting properties during rstore is not supported.
pub async fn restore(cx: app::Context, backup_name: Option<String>, restore_name: Option<String>) {
    // let backups = list_backups_api(&cx, false).await;
    let available_backups: Vec<BackupSummary> = list_backups_api(&cx, false)
        .await
        .into_iter()
        .filter(|b: &BackupSummary| b.to_owned().backup_status.unwrap() == "AVAILABLE")
        .collect();
    // let available_backups: Vec<BackupSummary> = backups.iter().filter(|b| b.backup_status.to_owned().unwrap() == "AVAILABLE").collect();
    if available_backups.is_empty() {
        app::bye(0, "No AVAILABLE state backup found for the table.");
    };

    let source_table_name = cx.effective_table_name();
    let backup_arn = match backup_name {
        Some(bname) => fetch_arn_from_backup_name(bname, available_backups),
        None => {
            let selection_texts: Vec<String> = available_backups
                .iter()
                .map(|b| {
                    format!(
                        "{} ({}, {} bytes)",
                        b.to_owned().backup_name.unwrap(),
                        epoch_to_rfc3339(b.backup_creation_date_time.unwrap()),
                        b.backup_size_bytes.unwrap()
                    )
                })
                .collect();

            debug!("available selections: {:#?}", selection_texts);

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select backup data to restore:")
                .default(0) /* &mut Select */
                .items(&selection_texts[..]) /* &mut Select */
                .interact() /* Result<usize, Error> */
                .unwrap();

            available_backups[selection].backup_arn.clone().unwrap()
        }
    };

    let epoch: u64 = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .expect("should be able to generate UNIX EPOCH")
        .as_secs();

    let target_table_name = match restore_name {
        None => format!("{}--restore-{}", source_table_name, epoch),
        Some(restore) => restore,
    };

    let ddb = DynamoDbClient::new(cx.effective_region());
    // https://docs.rs/rusoto_dynamodb/0.44.0/rusoto_dynamodb/struct.RestoreTableFromBackupInput.html
    let req: RestoreTableFromBackupInput = RestoreTableFromBackupInput {
        backup_arn: backup_arn.clone(),
        target_table_name,
        ..Default::default()
    };

    match ddb.restore_table_from_backup(req).await {
        Err(e) => {
            debug!("RestoreTableFromBackup API call got an error -- {:#?}", e);
            /* e.g. ... Possibly see "BackupInUse" error:
                [2020-08-14T13:16:07Z DEBUG dy::control] RestoreTableFromBackup API call got an error -- Service( BackupInUse( "Backup is being used to restore another table: arn:aws:dynamodb:us-west-2:111111111111:table/Music/backup/01527492829107-81b9b3dd",))
            */
        }
        Ok(res) => {
            debug!("Returned result: {:#?}", res);
            println!("Table restoration from: '{}' has been started", &backup_arn);
            let desc = res.table_description.unwrap();
            print_table_description(cx.effective_region(), desc);
        }
    }
}

/// Map "BilingModeSummary" field in table description returned from DynamoDB API,
/// into convenient mode name ("Provisioned" or "OnDemand")
pub fn extract_mode(bs: &Option<BillingModeSummary>) -> Mode {
    let provisioned_mode = Mode::Provisioned;
    let ondemand_mode = Mode::OnDemand;
    match bs {
        // if BillingModeSummary field doesn't exist, the table is Provisioned Mode.
        None => provisioned_mode,
        Some(x) => {
            if x.clone().billing_mode.unwrap() == ONDEMAND_API_SPEC {
                ondemand_mode
            } else {
                provisioned_mode
            }
        }
    }
}

/* =================================================
Private functions
================================================= */

/// Using Vec of String which is passed via command line,
/// generate KeySchemaElement(s) & AttributeDefinition(s), that are essential information to create DynamoDB tables or GSIs.
fn generate_essential_key_definitions(
    given_keys: &[String],
) -> (Vec<KeySchemaElement>, Vec<AttributeDefinition>) {
    let mut key_schema: Vec<KeySchemaElement> = vec![];
    let mut attribute_definitions: Vec<AttributeDefinition> = vec![];
    for (key_id, key_str) in given_keys.iter().enumerate() {
        let key_and_type = key_str.split(',').collect::<Vec<&str>>();
        if key_and_type.len() >= 3 {
            error!(
                "Invalid format for --keys option: '{}'. Valid format is '--keys myPk,S mySk,N'",
                &key_str
            );
            std::process::exit(1);
        }

        // assumes first given key is Partition key, and second given key is Sort key (if any).
        key_schema.push(KeySchemaElement {
            attribute_name: String::from(key_and_type[0]),
            key_type: if key_id == 0 {
                String::from("HASH")
            } else {
                String::from("RANGE")
            },
        });

        // If data type of key is omitted, dynein assumes it as String (S).
        attribute_definitions.push(AttributeDefinition {
            attribute_name: String::from(key_and_type[0]),
            attribute_type: if key_and_type.len() == 2 {
                key_and_type[1].to_uppercase()
            } else {
                String::from("S")
            },
        });
    }
    (key_schema, attribute_definitions)
}

/// Basically called by list_tables function, which is called from `$ dy list`.
/// To make ListTables API result reusable, separated API logic into this standalone function.
async fn list_tables_api(cx: app::Context) -> Vec<String> {
    let ddb = DynamoDbClient::new(cx.effective_region());
    let req: ListTablesInput = Default::default();
    match ddb.list_tables(req).await {
        Err(e) => {
            debug!("ListTables API call got an error -- {:#?}", e);
            error!("{}", e.to_string());
            std::process::exit(1);
        }
        // ListTables API returns blank array even if no table exists in a region.
        Ok(res) => res.table_names.expect("This message should not be shown"),
    }
}

/// This function is a private function that simply calls ListBackups API and return results
async fn list_backups_api(cx: &app::Context, all_tables: bool) -> Vec<BackupSummary> {
    let ddb = DynamoDbClient::new(cx.effective_region());
    let req: ListBackupsInput = ListBackupsInput {
        table_name: if all_tables {
            None
        } else {
            Some(cx.effective_table_name())
        },
        ..Default::default()
    };

    match ddb.list_backups(req).await {
        Err(e) => {
            debug!("ListBackups API call got an error -- {:#?}", e);
            // app::bye(1, &e.to_string()) // it doesn't meet return value requirement.
            println!("{}", &e.to_string());
            std::process::exit(1);
        }
        Ok(res) => res
            .backup_summaries
            .expect("backup result should have something"),
    }
}

fn fetch_arn_from_backup_name(
    backup_name: String,
    available_backups: Vec<BackupSummary>,
) -> String {
    available_backups
        .into_iter()
        .find(|b| b.to_owned().backup_name.unwrap() == backup_name) /* Option<BackupSummary */
        .unwrap() /* BackupSummary */
        .backup_arn /* Option<String> */
        .unwrap()
}

fn epoch_to_rfc3339(epoch: f64) -> String {
    let utc_datetime = NaiveDateTime::from_timestamp(epoch as i64, 0);
    DateTime::<Utc>::from_utc(utc_datetime, Utc).to_rfc3339()
}

/// Takes "Mode" enum and return exact string value required by DynamoDB API.
/// i.e. this function returns "PROVISIONED" or "PAY_PER_REQUEST".
fn mode_to_billing_mode_api_spec(mode: Mode) -> String {
    match mode {
        Mode::OnDemand => String::from(ONDEMAND_API_SPEC),
        Mode::Provisioned => String::from(PROVISIONED_API_SPEC),
    }
}

fn extract_capacity(
    mode: &Mode,
    cap_desc: &Option<ProvisionedThroughputDescription>,
) -> Option<PrintCapacityUnits> {
    if mode == &Mode::OnDemand {
        None
    } else {
        let desc = cap_desc.as_ref().unwrap();
        Some(PrintCapacityUnits {
            wcu: desc.write_capacity_units.unwrap(),
            rcu: desc.read_capacity_units.unwrap(),
        })
    }
}

trait IndexDesc {
    fn retrieve_index_name(&self) -> &Option<String>;
    fn retrieve_key_schema(&self) -> &Option<Vec<KeySchemaElement>>;
    fn extract_index_capacity(&self, m: &Mode) -> Option<PrintCapacityUnits>;
}

impl IndexDesc for GlobalSecondaryIndexDescription {
    fn retrieve_index_name(&self) -> &Option<String> {
        &self.index_name
    }
    fn retrieve_key_schema(&self) -> &Option<Vec<KeySchemaElement>> {
        &self.key_schema
    }
    fn extract_index_capacity(&self, m: &Mode) -> Option<PrintCapacityUnits> {
        if m == &Mode::OnDemand {
            None
        } else {
            extract_capacity(m, &self.provisioned_throughput)
        }
    }
}

impl IndexDesc for LocalSecondaryIndexDescription {
    fn retrieve_index_name(&self) -> &Option<String> {
        &self.index_name
    }
    fn retrieve_key_schema(&self) -> &Option<Vec<KeySchemaElement>> {
        &self.key_schema
    }
    fn extract_index_capacity(&self, _: &Mode) -> Option<PrintCapacityUnits> {
        None // Unlike GSI, LSI doesn't have it's own capacity.
    }
}

// FYI: https://grammarist.com/usage/indexes-indices/
fn extract_secondary_indexes<T: IndexDesc>(
    mode: &Mode,
    attr_defs: &[AttributeDefinition],
    option_indexes: Option<Vec<T>>,
) -> Option<Vec<PrintSecondaryIndex>> {
    if let Some(indexes) = option_indexes {
        let mut xs = Vec::<PrintSecondaryIndex>::new();
        for idx in &indexes {
            let ks = &idx.retrieve_key_schema().as_ref().unwrap();
            let idx = PrintSecondaryIndex {
                name: String::from(idx.retrieve_index_name().as_ref().unwrap()),
                schema: PrintPrimaryKeys {
                    pk: app::typed_key_for_schema("HASH", ks, attr_defs)
                        .expect("pk should exist")
                        .display(),
                    sk: app::typed_key_for_schema("RANGE", ks, attr_defs).map(|k| k.display()),
                },
                capacity: idx.extract_index_capacity(mode),
            };
            xs.push(idx);
        }
        Some(xs)
    } else {
        None
    }
}

fn extract_stream(arn: Option<String>, spec: Option<StreamSpecification>) -> Option<String> {
    if arn.is_none() {
        None
    } else {
        Some(format!(
            "{} ({})",
            arn.unwrap(),
            spec.unwrap().stream_view_type.unwrap()
        ))
    }
}
