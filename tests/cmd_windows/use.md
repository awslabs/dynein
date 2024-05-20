## dy use

```
$ dy use --help
Switch target table context. After you use the command you don't need to specify table every time, but you may overwrite the target table with --table (-t) option.

When you execute `use`, dynein retrieves table schema info via DescribeTable API
and stores it in ~/.dynein/ directory.

Usage: dy[EXE] use [OPTIONS] [TARGET_TABLE_TO_USE]

Arguments:
  [TARGET_TABLE_TO_USE]
          Target table name to use. Optionally you may specify the target table by --table (-t) option

Options:
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

$ dy help use
Switch target table context. After you use the command you don't need to specify table every time, but you may overwrite the target table with --table (-t) option.

When you execute `use`, dynein retrieves table schema info via DescribeTable API
and stores it in ~/.dynein/ directory.

Usage: dy[EXE] use [OPTIONS] [TARGET_TABLE_TO_USE]

Arguments:
  [TARGET_TABLE_TO_USE]
          Target table name to use. Optionally you may specify the target table by --table (-t) option

Options:
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
