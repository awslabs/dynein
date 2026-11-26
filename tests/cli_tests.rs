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

//! Snapshot tests for CLI output (mainly help messages).
//!
//! Each case runs `dy` with the given arguments and compares the output with
//! `tests/snapshots/cli/<file>`. A successful run is compared on stdout; a failing
//! run is compared on stderr. `[EXE]` in a snapshot matches `.exe` on Windows and
//! nothing elsewhere, so the same snapshots are used on every platform.
//!
//! To update snapshots, run `SNAPSHOTS=overwrite cargo test --test cli_tests`.

use snapbox::cmd::{cargo_bin, Command};
use snapbox::Data;
use std::path::Path;

/// (arguments, expected exit code, snapshot file name)
const CASES: &[(&[&str], i32, &str)] = &[
    (&["admin", "--help"], 0, "admin_help-flag.txt"),
    (&["help", "admin"], 0, "help_admin.txt"),
    (&["admin", "help"], 0, "admin_help.txt"),
    (
        &["admin", "create", "--help"],
        0,
        "admin_create_help-flag.txt",
    ),
    (
        &["admin", "create", "index", "--help"],
        0,
        "admin_create_index_help-flag.txt",
    ),
    (
        &["admin", "create", "table", "--help"],
        0,
        "admin_create_table_help-flag.txt",
    ),
    (
        &["admin", "delete", "--help"],
        0,
        "admin_delete_help-flag.txt",
    ),
    (
        &["admin", "delete", "table", "--help"],
        0,
        "admin_delete_table_help-flag.txt",
    ),
    (&["admin", "desc", "--help"], 0, "admin_desc_help-flag.txt"),
    (&["admin", "list", "--help"], 0, "admin_list_help-flag.txt"),
    (
        &["admin", "update", "--help"],
        0,
        "admin_update_help-flag.txt",
    ),
    (
        &["admin", "update", "table", "--help"],
        0,
        "admin_update_table_help-flag.txt",
    ),
    (&["backup", "--help"], 0, "backup_help-flag.txt"),
    (&["help", "backup"], 0, "help_backup.txt"),
    (&["bootstrap", "--help"], 0, "bootstrap_help-flag.txt"),
    (&["help", "bootstrap"], 0, "help_bootstrap.txt"),
    (&["bwrite", "--help"], 0, "bwrite_help-flag.txt"),
    (&["help", "bwrite"], 0, "help_bwrite.txt"),
    (&["config", "--help"], 0, "config_help-flag.txt"),
    (&["help", "config"], 0, "help_config.txt"),
    (
        &["config", "clear", "--help"],
        0,
        "config_clear_help-flag.txt",
    ),
    (
        &["config", "dump", "--help"],
        0,
        "config_dump_help-flag.txt",
    ),
    (&["del", "--help"], 0, "del_help-flag.txt"),
    (&["help", "del"], 0, "help_del.txt"),
    (&["desc", "--help"], 0, "desc_help-flag.txt"),
    (&["help", "desc"], 0, "help_desc.txt"),
    (&["export", "--help"], 0, "export_help-flag.txt"),
    (&["help", "export"], 0, "help_export.txt"),
    (&["get", "--help"], 0, "get_help-flag.txt"),
    (&["help", "get"], 0, "help_get.txt"),
    (&["--help"], 0, "help-flag.txt"),
    (&["help"], 0, "help.txt"),
    (&["help", "--help"], 2, "help_help-flag.txt"),
    (&["import", "--help"], 0, "import_help-flag.txt"),
    (&["help", "import"], 0, "help_import.txt"),
    (&["list", "--help"], 0, "list_help-flag.txt"),
    (&["help", "list"], 0, "help_list.txt"),
    (&["put", "--help"], 0, "put_help-flag.txt"),
    (&["help", "put"], 0, "help_put.txt"),
    (&["query", "--help"], 0, "query_help-flag.txt"),
    (&["help", "query"], 0, "help_query.txt"),
    (&["restore", "--help"], 0, "restore_help-flag.txt"),
    (&["help", "restore"], 0, "help_restore.txt"),
    (&["scan", "--help"], 0, "scan_help-flag.txt"),
    (&["help", "scan"], 0, "help_scan.txt"),
    (&["upd", "--help"], 0, "upd_help-flag.txt"),
    (&["help", "upd"], 0, "help_upd.txt"),
    (&["use", "--help"], 0, "use_help-flag.txt"),
    (&["help", "use"], 0, "help_use.txt"),
];

#[test]
fn cli_tests() {
    let snapshot_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/cli");
    for (args, code, snapshot) in CASES {
        let expected = Data::read_from(&snapshot_dir.join(snapshot), None);
        let assert = Command::new(cargo_bin!("dy"))
            .args(*args)
            .assert()
            .code(*code);
        if *code == 0 {
            assert.stdout_eq(expected).stderr_eq("");
        } else {
            assert.stdout_eq("").stderr_eq(expected);
        }
    }
}
