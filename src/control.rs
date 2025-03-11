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
use aws_sdk_dynamodb::{
    types::{
        BackupStatus, BackupSummary, BillingMode, CreateGlobalSecondaryIndexAction,
        GlobalSecondaryIndexUpdate, Projection, ProjectionType, ProvisionedThroughput,
        TableDescription,
    },
    Client as DynamoDbSdkClient,
};
use aws_sdk_ec2::Client as Ec2SdkClient;
use futures::future::join_all;
use log::{debug, error};
use std::borrow::Cow::{Borrowed, Owned};
use std::{
    io::{self, Error as IOError, Write},
    time,
};

use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use tabwriter::TabWriter;

use super::app;
use super::ddb::table;

/* =================================================
Public functions
================================================= */

pub async fn list_tables_all_regions(cx: &app::Context) {
    // get all regions from us-east-1 regardless specified region
    let config = cx
        .clone()
        .with_region("us-east-1")
        .effective_sdk_config()
        .await;
    let ec2 = Ec2SdkClient::new(&config);
    match ec2.describe_regions().send().await {
        Err(e) => {
            app::bye_with_sdk_error(1, e);
        }
        Ok(res) => {
            join_all(
                res.regions
                    .expect("regions should exist") // Vec<Region>
                    .iter()
                    .map(|r| list_tables(cx, Some(r.region_name.as_ref().unwrap()))),
            )
            .await;

            if cx.is_local().await {
                list_tables(cx, None).await;
            }
        }
    };
}

pub async fn list_tables(cx: &app::Context, override_region: Option<&str>) {
    let table_names = list_tables_api(cx, override_region).await;
    let region = cx.effective_region().await.to_string();

    println!("DynamoDB tables in region: {}", region);
    if table_names.is_empty() {
        return println!("  No table in this region.");
    }

    if let Some(table_in_config) = cx.cached_using_table_schema().await {
        for table_name in table_names {
            if region == table_in_config.region && table_name == table_in_config.name {
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
pub async fn describe_all_tables(cx: &app::Context) {
    let table_names = list_tables_api(cx, None).await;
    join_all(table_names.into_iter().map(|t| describe_table(cx, Some(t)))).await;
}

/// Executed when you call `$ dy desc (table)`. Retrieve TableDescription via describe_table_api function,
/// then print them in convenient way using table::print_table_description function (default/yaml).
pub async fn describe_table(cx: &app::Context, target_table_to_desc: Option<String>) {
    debug!("context: {:#?}", &cx);
    debug!("positional arg table name: {:?}", &target_table_to_desc);
    let new_context = if let Some(t) = target_table_to_desc {
        Owned(cx.clone().with_table(&t))
    } else {
        Borrowed(cx)
    };

    let desc: TableDescription =
        describe_table_api(new_context.as_ref(), new_context.effective_table_name()).await;
    debug!(
        "Retrieved table to describe is: '{}' table in '{}' region.",
        new_context.effective_table_name(),
        new_context.effective_region().await.as_ref()
    );

    // save described table info into cache for future use.
    // Note that when this functiono is called from describe_all_tables, not all tables would be cached as calls are parallel.
    match app::insert_to_table_cache(new_context.as_ref(), &desc).await {
        Ok(_) => debug!("Described table schema was written to the cache file."),
        Err(e) => println!(
            "Failed to write table schema to the cache with follwoing error: {:?}",
            e
        ),
    };

    match new_context.output.as_deref() {
        None | Some("yaml") => {
            table::print_table_description(new_context.effective_region().await.as_ref(), &desc)
        }
        // Some("raw") => println!("{:#?}", desc),
        Some(_) => {
            println!("ERROR: unsupported output type.");
            std::process::exit(1);
        }
    }
}

/// Originally intended to be called by describe_table function, which is called from `$ dy desc`,
/// however it turned out that DescribeTable API result is useful in various logic, separated API into this standalone function.
pub async fn describe_table_api(cx: &app::Context, table_name: String) -> TableDescription {
    let region = cx.effective_region().await;
    let config = cx.effective_sdk_config_with_region(region.as_ref()).await;
    let ddb = &cx.ddb_client;

    match ddb.describe_table().table_name(table_name).send().await {
        Err(e) => {
            debug!("DescribeTable API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
        }
        Ok(res) => {
            let desc: TableDescription = res.table.expect("This message should not be shown.");
            debug!("Received DescribeTable Result: {:?}\n", desc);
            desc
        }
    }
}

/// This function is designed to be called from dynein command, mapped in main.rs.
/// Note that it simply ignores --table option if specified. Newly created table name should be given by the 1st argument "name".
pub async fn create_table(cx: &app::Context, name: String, given_keys: Vec<String>) {
    if given_keys.is_empty() || given_keys.len() >= 3 {
        error!("You should pass one or two key definitions with --keys option");
        std::process::exit(1);
    };

    match create_table_api(cx, name, given_keys).await {
        Ok(desc) => table::print_table_description(cx.effective_region().await.as_ref(), &desc),
        Err(e) => {
            debug!("CreateTable API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
        }
    }
}

pub async fn create_table_api(
    cx: &app::Context,
    name: String,
    given_keys: Vec<String>,
) -> Result<
    TableDescription,
    aws_sdk_dynamodb::error::SdkError<aws_sdk_dynamodb::operation::create_table::CreateTableError>,
> {
    debug!(
        "Trying to create a table '{}' with keys '{:?}'",
        &name, &given_keys
    );

    let (key_schema, attribute_definitions) =
        table::generate_essential_key_definitions(&given_keys);

    let ddb = &cx.ddb_client;

    ddb.create_table()
        .table_name(name)
        .billing_mode(BillingMode::PayPerRequest)
        .set_key_schema(Some(key_schema))
        .set_attribute_definitions(Some(attribute_definitions))
        .send()
        .await
        .map(|res| {
            res.table_description
                .expect("Table Description returned from API should be valid.")
        })
}

pub async fn create_index(cx: &app::Context, index_name: String, given_keys: Vec<String>) {
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

    let (key_schema, attribute_definitions) =
        table::generate_essential_key_definitions(&given_keys);

    let ddb = &cx.ddb_client;

    let create_gsi_action = CreateGlobalSecondaryIndexAction::builder()
        .index_name(index_name)
        .set_key_schema(Some(key_schema))
        .projection(
            Projection::builder()
                .projection_type(ProjectionType::All)
                .build(),
        )
        .set_provisioned_throughput(None) // TODO: assign default rcu/wcu if base table is Provisioned mode. currently it works only for OnDemand talbe.
        .build()
        .unwrap();

    let gsi_update = GlobalSecondaryIndexUpdate::builder()
        .create(create_gsi_action)
        .build();

    match ddb
        .update_table()
        .table_name(cx.effective_table_name())
        .set_attribute_definitions(Some(attribute_definitions))
        .global_secondary_index_updates(gsi_update)
        .send()
        .await
    {
        Err(e) => {
            debug!("UpdateTable API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
        }
        Ok(res) => {
            debug!("Returned result: {:#?}", res);
            table::print_table_description(
                cx.effective_region().await.as_ref(),
                &res.table_description.unwrap(),
            );
        }
    }
}

pub async fn update_table(
    cx: &app::Context,
    table_name_to_update: String,
    mode_string: Option<String>,
    wcu: Option<i64>,
    rcu: Option<i64>,
) {
    // Retrieve TableDescription of the table to update, current (before update) status.
    let desc: TableDescription = describe_table_api(cx, table_name_to_update.clone()).await;

    // Map given string into "Mode" enum. Note that in cmd.rs clap already limits acceptable values.
    let switching_to_mode: Option<table::Mode> = match mode_string {
        None => None,
        Some(ms) => match ms.as_str() {
            "provisioned" => Some(table::Mode::Provisioned),
            "ondemand"    => Some(table::Mode::OnDemand),
            _ => panic!("You shouldn't see this message as --mode can takes only 'provisioned' or 'ondemand'."),
        },
    };

    // Configure ProvisionedThroughput struct based on argumsnts (mode/wcu/rcu).
    let provisioned_throughput: Option<ProvisionedThroughput> = match &switching_to_mode {
        // when --mode is not given, no mode switch happens. Check the table's current mode.
        None => {
            match table::extract_mode(&desc.billing_mode_summary) {
                // When currently OnDemand mode and you're not going to change the it, set None for CU.
                table::Mode::OnDemand => {
                    if wcu.is_some() || rcu.is_some() {
                        println!("Ignoring --rcu/--wcu options as the table mode is OnDemand.");
                    };
                    None
                }
                // When currently Provisioned mode and you're not going to change the it,
                // pass given rcu/wcu, and use current values if missing. Provisioned table should have valid capacity units so unwrap() here.
                table::Mode::Provisioned => Some(
                    ProvisionedThroughput::builder()
                        .read_capacity_units(rcu.unwrap_or_else(|| {
                            desc.provisioned_throughput
                                .as_ref()
                                .unwrap()
                                .read_capacity_units
                                .unwrap()
                        }))
                        .write_capacity_units(wcu.unwrap_or_else(|| {
                            desc.provisioned_throughput
                                .as_ref()
                                .unwrap()
                                .write_capacity_units
                                .unwrap()
                        }))
                        .build()
                        .unwrap(),
                ),
            }
        }
        // When the user trying to switch mode.
        Some(target_mode) => match target_mode {
            // when switching Provisioned->OnDemand mode, ProvisionedThroughput can be None.
            table::Mode::OnDemand => {
                if wcu.is_some() || rcu.is_some() {
                    println!("Ignoring --rcu/--wcu options as --mode ondemand.");
                };
                None
            }
            // when switching OnDemand->Provisioned mode, set given wcu/rcu, fill with "5" as a default if not given.
            table::Mode::Provisioned => Some(
                ProvisionedThroughput::builder()
                    .read_capacity_units(rcu.unwrap_or(5))
                    .write_capacity_units(wcu.unwrap_or(5))
                    .build()
                    .unwrap(),
            ),
        },
    };

    // TODO: support updating CU of the table with GSI. If the table has GSIs, you must specify CU for them at the same time.
    // error message: One or more parameter values were invalid: ProvisionedThroughput must be specified for index: xyz_index,abc_index2
    //   if table has gsi
    //     build GlobalSecondaryIndexUpdates { [... current values ...] }

    match update_table_api(
        cx,
        table_name_to_update,
        switching_to_mode,
        provisioned_throughput,
    )
    .await
    {
        Ok(desc) => table::print_table_description(cx.effective_region().await.as_ref(), &desc),
        Err(e) => {
            debug!("UpdateTable API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
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
    cx: &app::Context,
    table_name_to_update: String,
    switching_to_mode: Option<table::Mode>,
    provisioned_throughput: Option<ProvisionedThroughput>,
) -> Result<
    TableDescription,
    aws_sdk_dynamodb::error::SdkError<aws_sdk_dynamodb::operation::update_table::UpdateTableError>,
> {
    debug!("Trying to update the table '{}'.", &table_name_to_update);

    let ddb = &cx.ddb_client;

    ddb.update_table()
        .table_name(table_name_to_update)
        .set_billing_mode(switching_to_mode.map(|v| v.into()))
        .set_provisioned_throughput(provisioned_throughput)
        .send()
        .await
        .map(|res| {
            res.table_description
                .expect("Table Description returned from API should be valid.")
        })
}

pub async fn delete_table(cx: &app::Context, name: String, skip_confirmation: bool) {
    debug!("Trying to delete a table '{}'", &name);

    let msg = format!("You're trying to delete a table '{}'. Are you OK?", &name);
    if !skip_confirmation && !Confirm::new().with_prompt(&msg).interact().unwrap() {
        println!("The table delete operation has been canceled.");
        return;
    }

    let ddb = &cx.ddb_client;

    match ddb.delete_table().table_name(name).send().await {
        Err(e) => {
            debug!("DeleteTable API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
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
pub async fn backup(cx: &app::Context, all_tables: bool) {
    // this "backup" function is called only when --list is NOT given. So, --all-tables would be ignored.
    if all_tables {
        println!("NOTE: --all-tables option is ignored without --list option. Just trying to create a backup for the target table...")
    };

    let table_name = cx.effective_table_name();
    debug!("Taking a backof of the table '{}'", table_name);
    let epoch: u64 = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .expect("should be able to generate UNIX EPOCH")
        .as_secs();

    let ddb = &cx.ddb_client;

    let req = ddb
        .create_backup()
        .table_name(&table_name)
        .backup_name(format!("{}--dynein-{}", table_name, epoch));
    debug!("backup req: {:?}", req);

    match req.send().await {
        Err(e) => {
            debug!("CreateBackup API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
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
pub async fn list_backups(cx: &app::Context, all_tables: bool) -> Result<(), IOError> {
    let backups = list_backups_api(cx, all_tables).await;
    let mut tw = TabWriter::new(io::stdout());
    // First defining header
    tw.write_all(
        ((["Table", "Status", "CreatedAt", "BackupName (size)"].join("\t")) + "\n").as_bytes(),
    )?;
    for backup in backups {
        let line = [
            backup.table_name.expect("table name should exist"),
            backup
                .backup_status
                .expect("status should exist")
                .as_str()
                .to_string(),
            table::epoch_to_rfc3339(
                backup
                    .backup_creation_date_time
                    .expect("creation date should exist")
                    .as_secs_f64(),
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
pub async fn restore(cx: &app::Context, backup_name: Option<String>, restore_name: Option<String>) {
    // let backups = list_backups_api(&cx, false).await;
    let available_backups: Vec<BackupSummary> = list_backups_api(cx, false)
        .await
        .into_iter()
        .filter(|b: &BackupSummary| b.to_owned().backup_status == Some(BackupStatus::Available))
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
                        table::epoch_to_rfc3339(b.backup_creation_date_time.unwrap().as_secs_f64()),
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

    let ddb = &cx.ddb_client;

    match ddb
        .restore_table_from_backup()
        .backup_arn(backup_arn.clone())
        .target_table_name(target_table_name)
        .send()
        .await
    {
        Err(e) => {
            debug!("RestoreTableFromBackup API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
        }
        Ok(res) => {
            debug!("Returned result: {:#?}", res);
            println!("Table restoration from: '{}' has been started", &backup_arn);
            let desc = res.table_description.unwrap();
            table::print_table_description(cx.effective_region().await.as_ref(), &desc);
        }
    }
}

/* =================================================
Private functions
================================================= */

/// Basically called by list_tables function, which is called from `$ dy list`.
/// To make ListTables API result reusable, separated API logic into this standalone function.
async fn list_tables_api(cx: &app::Context, override_region: Option<&str>) -> Vec<String> {
    let config = if let Some(override_region) = override_region {
        cx.effective_sdk_config_with_region(override_region).await
    } else {
        cx.effective_sdk_config().await
    };
    let ddb = &cx.ddb_client;

    match ddb.list_tables().send().await {
        Err(e) => {
            debug!("ListTables API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
        }
        // ListTables API returns blank array even if no table exists in a region.
        Ok(res) => res.table_names.expect("This message should not be shown"),
    }
}

/// This function is a private function that simply calls ListBackups API and return results
async fn list_backups_api(cx: &app::Context, all_tables: bool) -> Vec<BackupSummary> {
    let ddb = &cx.ddb_client;

    let mut req = ddb.list_backups();
    if !all_tables {
        req = req.table_name(cx.effective_table_name());
    }

    match req.send().await {
        Err(e) => {
            debug!("ListBackups API call got an error -- {:#?}", e);
            app::bye_with_sdk_error(1, e);
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
