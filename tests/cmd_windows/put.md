## dy put

```
$ dy put --help
Create a new item, or replace an existing item. [API: PutItem]

Usage: dy[EXE] put [OPTIONS] <PVAL> [SVAL]

Arguments:
  <PVAL>  Partition Key of the target item
  [SVAL]  Sort Key of the target item (if any)

Options:
  -i, --item <ITEM>      Additional attributes put into the item, which should be valid JSON.
                         e.g. --item '{"name": "John", "age": 18, "like": ["Apple", "Banana"]}'
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy help put
Create a new item, or replace an existing item. [API: PutItem]

Usage: dy[EXE] put [OPTIONS] <PVAL> [SVAL]

Arguments:
  <PVAL>  Partition Key of the target item
  [SVAL]  Sort Key of the target item (if any)

Options:
  -i, --item <ITEM>      Additional attributes put into the item, which should be valid JSON.
                         e.g. --item '{"name": "John", "age": 18, "like": ["Apple", "Banana"]}'
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

```
