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

use std::error::Error;

extern crate chrono;
extern crate env_logger;
extern crate log;
extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate serde_json;
extern crate serde_yaml;

use log::{debug};

mod app;
mod batch;
mod bootstrap;
mod cmd;
mod control;
mod data;
mod transfer;


/* =================================================
   main() function
   =================================================
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let c = cmd::initialize_from_args();
    debug!("Command details: {:?}", c);

    // when --region <region-name e.g. ap-northeast-1>, use the region. when --region local, use DynamoDB local.
    // --region/--table option can be passed as a top-level or subcommand-level (i.e. global).
    let mut context = app::Context {
        region: None,
        config: Some(app::load_or_touch_config_file(true)?),
        cache:  Some(app::load_or_touch_cache_file(true)?),
        overwritten_region: app::region_from_str(c.region),
        overwritten_table_name: c.table,
        output: None,
    };
    debug!("Initial command context: {:?}", &context);

    match c.child {
        cmd::Sub::Admin { grandchild } => match grandchild {
            cmd::AdminSub::List { all_regions } => {
                if all_regions { control::list_tables_all_regions(context).await }
                else { control::list_tables(context).await }
            },
            cmd::AdminSub::Desc { all_tables, output } => {
                context.output = output;
                if all_tables { control::describe_all_tables(context).await }
                else { control::describe_table(context).await }
            },
            cmd::AdminSub::Create { target_type } => match target_type {
                cmd::CreateSub::Table { new_table_name, keys } => control::create_table(context, new_table_name, keys).await,
                cmd::CreateSub::Index { index_name, keys } => control::create_index(context, index_name, keys).await,
            },
            cmd::AdminSub::Update { target_type } => match target_type {
                cmd::UpdateSub::Table { table_name_to_update, mode, wcu, rcu } => control::update_table(context, table_name_to_update, mode, wcu, rcu).await,
            },
            cmd::AdminSub::Delete { target_type } => match target_type {
                cmd::DeleteSub::Table { table_name_to_delete, yes } => control::delete_table(context, table_name_to_delete, yes).await,
            },
        },
 
        cmd::Sub::Scan   { index, consistent_read, attributes, keys_only, limit, output } => {
            context.output = output;
            data::scan(context, index, consistent_read, &attributes, keys_only, limit).await
        },
        cmd::Sub::Query  { pval, sort_key_expression, index, attributes, consistent_read, keys_only, output } => {
            context.output = output;
            data::query(context, pval, sort_key_expression, index, consistent_read, &attributes, keys_only).await
        },
        cmd::Sub::Get    { pval, sval, consistent_read, output } => {
            context.output = output;
            data::get_item(context, pval, sval, consistent_read).await
        },
        cmd::Sub::Put    { pval, sval, item } => data::put_item(context, pval, sval, item).await,
        cmd::Sub::Del    { pval, sval } => data::delete_item(context, pval, sval).await,
        cmd::Sub::Upd    { pval, sval, set, remove, atomic_counter } => {
            if let Some(target) = atomic_counter { data::atomic_counter(context, pval, sval, set, remove, target).await; }
            else { data::update_item(context, pval, sval, set, remove).await; }
        },
        cmd::Sub::Bwrite { input } => { batch::batch_write_item(context, input).await? },

        cmd::Sub::List { all_regions } => {
            if all_regions { control::list_tables_all_regions(context).await }
            else { control::list_tables(context).await }
        },
        cmd::Sub::Desc { all_tables, output } => {
            context.output = output;
            if all_tables { control::describe_all_tables(context).await }
            else { control::describe_table(context).await }
        },
        cmd::Sub::Use { target_table_to_use }  => app::use_table(&context, target_table_to_use).await?,
        cmd::Sub::Config { grandchild } => match grandchild {
            cmd::ConfigSub::Dump  => {
                println!("{}", serde_yaml::to_string(&app::load_or_touch_cache_file(true)?)?);
                println!("{}", serde_yaml::to_string(&app::load_or_touch_config_file(true)?)?);
            },
            cmd::ConfigSub::Clear => app::remove_dynein_files()?,
        },

        cmd::Sub::Bootstrap { list, sample } => {
            if list { bootstrap::list_samples() }
            else { bootstrap::launch_sample(context, sample).await? }  // sample can be None
        },

        cmd::Sub::Export { attributes, keys_only, output_file, format } => transfer::export(context, attributes, keys_only, output_file, format).await?,
        cmd::Sub::Import { input_file, format } => transfer::import(context, input_file, format).await?,
        cmd::Sub::Backup { list, all_tables } => {
            if list { control::list_backups(context, all_tables).await? }
            else    { control::backup(context, all_tables /* all_tables is simply ignored for "backup" */).await }
        },
        cmd::Sub::Restore { backup_name, restore_name } => control::restore(context, backup_name, restore_name).await,
    }

    Ok(())
}
