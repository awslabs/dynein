## dy desc

```
$ dy desc --help
Show detailed information of a table. [API: DescribeTable]

Usage: dy[EXE] desc [OPTIONS] [TARGET_TABLE_TO_DESC]

Arguments:
  [TARGET_TABLE_TO_DESC]  Target table name. Optionally you may specify the target table by --table (-t) option

Options:
      --all-tables       Show details of all tables in the region
  -o, --output <OUTPUT>  Switch output format. [possible values: yaml]
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy help desc
Show detailed information of a table. [API: DescribeTable]

Usage: dy[EXE] desc [OPTIONS] [TARGET_TABLE_TO_DESC]

Arguments:
  [TARGET_TABLE_TO_DESC]  Target table name. Optionally you may specify the target table by --table (-t) option

Options:
      --all-tables       Show details of all tables in the region
  -o, --output <OUTPUT>  Switch output format. [possible values: yaml]
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

```
