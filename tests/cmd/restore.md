## dy restore

```
$ dy restore --help
Restore a DynamoDB table from backup data

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

Usage: dy restore [OPTIONS]

Options:
  -b, --backup-name <BACKUP_NAME>
          Specify backup file. If not specified you can select it interactively.

      --restore-name <RESTORE_NAME>
          Name of the newly restored table. If not specified, default naming rule "<source-table-name>-restore-<timestamp>" would be used.

  -r, --region <REGION>
          The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
          You can use --region option in both top-level and subcommand-level.

  -p, --port <PORT>
          Specify the port number. This option has an effect only when `--region local` is used.

  -t, --table <TABLE>
          Target table of the operation. You can use --table option in both top-level and subcommand-level.
          You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.

  -h, --help
          Print help (see a summary with '-h')

$ dy help restore
Restore a DynamoDB table from backup data

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

Usage: dy restore [OPTIONS]

Options:
  -b, --backup-name <BACKUP_NAME>
          Specify backup file. If not specified you can select it interactively.

      --restore-name <RESTORE_NAME>
          Name of the newly restored table. If not specified, default naming rule "<source-table-name>-restore-<timestamp>" would be used.

  -r, --region <REGION>
          The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
          You can use --region option in both top-level and subcommand-level.

  -p, --port <PORT>
          Specify the port number. This option has an effect only when `--region local` is used.

  -t, --table <TABLE>
          Target table of the operation. You can use --table option in both top-level and subcommand-level.
          You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.

  -h, --help
          Print help (see a summary with '-h')

```
