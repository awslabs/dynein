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

use crate::data::QueryParams;
use brotli::Decompressor;
use std::io::{stdout, Cursor};

use log::debug;
use std::error::Error;

extern crate pest;

#[macro_use]
extern crate pest_derive;

mod app;
mod batch;
mod bootstrap;
mod cmd;
mod control;
mod data;
mod parser;
mod shell;
mod transfer;

/* =================================================
   helper functions
   =================================================
*/
async fn dispatch(context: &mut app::Context, subcommand: cmd::Sub) -> Result<(), Box<dyn Error>> {
    match subcommand {
        cmd::Sub::Admin { grandchild } => match grandchild {
            cmd::AdminSub::List { all_regions } => {
                if all_regions {
                    control::list_tables_all_regions(context.clone()).await
                } else {
                    control::list_tables(context.clone()).await
                }
            }
            cmd::AdminSub::Desc {
                target_table_to_desc,
                all_tables,
                output,
            } => {
                context.output = output;
                if all_tables {
                    control::describe_all_tables(context.clone()).await
                } else {
                    control::describe_table(context.clone(), target_table_to_desc).await
                }
            }
            cmd::AdminSub::Create { target_type } => match target_type {
                cmd::CreateSub::Table {
                    new_table_name,
                    keys,
                } => control::create_table(context.clone(), new_table_name, keys).await,
                cmd::CreateSub::Index { index_name, keys } => {
                    control::create_index(context.clone(), index_name, keys).await
                }
            },
            cmd::AdminSub::Update { target_type } => match target_type {
                cmd::UpdateSub::Table {
                    table_name_to_update,
                    mode,
                    wcu,
                    rcu,
                } => {
                    control::update_table(context.clone(), table_name_to_update, mode, wcu, rcu)
                        .await
                }
            },
            cmd::AdminSub::Delete { target_type } => match target_type {
                cmd::DeleteSub::Table {
                    table_name_to_delete,
                    yes,
                } => control::delete_table(context.clone(), table_name_to_delete, yes).await,
            },
            cmd::AdminSub::Apply { dev } => {
                if dev {
                    todo!()
                } else {
                    println!("not yet implemented")
                }
            }
        },

        cmd::Sub::Scan {
            index,
            consistent_read,
            attributes,
            keys_only,
            limit,
            output,
        } => {
            context.output = output;
            data::scan(
                context.clone(),
                index,
                consistent_read,
                &attributes,
                keys_only,
                limit,
            )
            .await
        }
        cmd::Sub::Query {
            pval,
            sort_key_expression,
            index,
            limit,
            attributes,
            consistent_read,
            keys_only,
            descending,
            strict,
            non_strict,
            output,
        } => {
            context.output = output;
            if strict || non_strict {
                context.should_strict_for_query = Some(strict || !non_strict)
            }
            data::query(
                context.clone(),
                QueryParams {
                    pval,
                    sort_key_expression,
                    index,
                    limit,
                    consistent_read,
                    descending,
                    attributes,
                    keys_only,
                },
            )
            .await
        }
        cmd::Sub::Get {
            pval,
            sval,
            consistent_read,
            output,
        } => {
            context.output = output;
            data::get_item(context.clone(), pval, sval, consistent_read).await
        }
        cmd::Sub::Put { pval, sval, item } => {
            data::put_item(context.clone(), pval, sval, item).await
        }
        cmd::Sub::Del { pval, sval } => data::delete_item(context.clone(), pval, sval).await,
        cmd::Sub::Upd {
            pval,
            sval,
            set,
            remove,
            atomic_counter,
        } => {
            if let Some(target) = atomic_counter {
                data::atomic_counter(context.clone(), pval, sval, set, remove, target).await;
            } else {
                data::update_item(context.clone(), pval, sval, set, remove).await;
            }
        }
        cmd::Sub::Bwrite { puts, dels, input } => {
            batch::batch_write_item(context.clone(), puts, dels, input).await?
        }

        cmd::Sub::List { all_regions } => {
            if all_regions {
                control::list_tables_all_regions(context.clone()).await
            } else {
                control::list_tables(context.clone()).await
            }
        }
        cmd::Sub::Desc {
            target_table_to_desc,
            all_tables,
            output,
        } => {
            context.output = output;
            if all_tables {
                control::describe_all_tables(context.clone()).await
            } else {
                control::describe_table(context.clone(), target_table_to_desc).await
            }
        }
        cmd::Sub::Use {
            target_table_to_use,
        } => app::use_table(context, target_table_to_use).await?,
        cmd::Sub::Config { grandchild } => match grandchild {
            cmd::ConfigSub::Dump => {
                println!(
                    "{}",
                    serde_yaml::to_string(&app::load_or_touch_cache_file(true)?)?
                );
                println!(
                    "{}",
                    serde_yaml::to_string(&app::load_or_touch_config_file(true)?)?
                );
            }
            cmd::ConfigSub::Clear => app::remove_dynein_files()?,
        },

        cmd::Sub::Bootstrap { list, sample } => {
            if list {
                bootstrap::list_samples()
            } else {
                bootstrap::launch_sample(context.clone(), sample).await?
            } // sample can be None
        }

        cmd::Sub::Export {
            attributes,
            keys_only,
            output_file,
            format,
        } => transfer::export(context.clone(), attributes, keys_only, output_file, format).await?,
        cmd::Sub::Import {
            input_file,
            format,
            enable_set_inference,
        } => transfer::import(context.clone(), input_file, format, enable_set_inference).await?,
        cmd::Sub::Backup { list, all_tables } => {
            if list {
                control::list_backups(context.clone(), all_tables).await?
            } else {
                control::backup(
                    context.clone(),
                    all_tables, /* all_tables is simply ignored for "backup" */
                )
                .await
            }
        }
        cmd::Sub::Restore {
            backup_name,
            restore_name,
        } => control::restore(context.clone(), backup_name, restore_name).await,
    }
    Ok(())
}

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
    let mut context = app::Context::new(c.region, c.port, c.table)?;
    debug!("Initial command context: {:?}", &context);

    if let Some(child) = c.child {
        // subcommand
        dispatch(&mut context, child).await?
    } else if c.shell {
        // shell mode
        use shell::BuiltinCommands;
        use shell::ShellInput::*;
        use std::io::stdin;

        let input = stdin();
        let mut reader = shell::ShellReader::new(&input);
        loop {
            let child = reader.read_line()?;
            match child {
                Builtin(BuiltinCommands::Exit) => break,
                Eof => break,
                Command(child) => {
                    if let Err(e) = dispatch(&mut context, child).await {
                        eprintln!("{}", e)
                    }
                }
                ParseError(_) => {
                    // do nothing because read_line already handles the error
                }
            }
        }
    } else if c.third_party_attribution {
        // Load 3rd party attribution file
        let compressed_data = include_bytes!("./resources/attribution/ThirdPartyAttribution.br");
        let cursor = Cursor::new(compressed_data);
        let mut decompressor = Decompressor::new(cursor, 4096);

        let mut stdout = stdout().lock();
        std::io::copy(&mut decompressor, &mut stdout)?;
    } else {
        // Neiter subcommand nor --shell specified
        use structopt::StructOpt;
        eprintln!("Invalid argument: please specify a subcommand or '--shell'");
        cmd::Dynein::clap().print_help()?;
        std::process::exit(1);
    }

    Ok(())
}
