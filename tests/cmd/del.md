## dy del

```
$ dy del --help
dy-del 0.2.1
Delete an existing item. [API: DeleteItem]

USAGE:
    dy del [OPTIONS] <pval> [sval]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <pval>    Partition Key of the target item
    <sval>    Sort Key of the target item (if any)

$ dy help del
dy-del 0.2.1
Delete an existing item. [API: DeleteItem]

USAGE:
    dy del [OPTIONS] <pval> [sval]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <pval>    Partition Key of the target item
    <sval>    Sort Key of the target item (if any)

```
