## dy del

```
$ dy del --help
Delete an existing item. [API: DeleteItem]

Usage: dy del [OPTIONS] <PVAL> [SVAL]

Arguments:
  <PVAL>  Partition Key of the target item
  [SVAL]  Sort Key of the target item (if any)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help             Print help

$ dy help del
Delete an existing item. [API: DeleteItem]

Usage: dy del [OPTIONS] <PVAL> [SVAL]

Arguments:
  <PVAL>  Partition Key of the target item
  [SVAL]  Sort Key of the target item (if any)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help             Print help

```
