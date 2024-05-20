## dy backup

```
$ dy backup --help
Take backup of a DynamoDB table using on-demand backup

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

Usage: dy[EXE] backup [OPTIONS]

Options:
  -l, --list
          List existing DynamoDB backups

      --all-tables
          List backups for all tables in the region

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

$ dy help backup
Take backup of a DynamoDB table using on-demand backup

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

Usage: dy[EXE] backup [OPTIONS]

Options:
  -l, --list
          List existing DynamoDB backups

      --all-tables
          List backups for all tables in the region

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
