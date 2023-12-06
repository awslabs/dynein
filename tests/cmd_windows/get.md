## dy get

```
$ dy get --help
dy[EXE]-get 0.2.1
Retrieve an item by specifying primary key(s). [API: GetItem]

USAGE:
    dy[EXE] get [FLAGS] [OPTIONS] <pval> [sval]

FLAGS:
        --consistent-read    Strong consistent read - to make sure retrieve the most up-to-date data. By default
                             (false), eventual consistent reads would occur.
                             https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
    -h, --help               Prints help information
    -V, --version            Prints version information

OPTIONS:
    -o, --output <output>    Switch output format [possible values: json, yaml, raw]
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <pval>    Partition Key of the target item
    <sval>    Sort Key of the target item (if any)

$ dy help get
dy[EXE]-get 0.2.1
Retrieve an item by specifying primary key(s). [API: GetItem]

USAGE:
    dy[EXE] get [FLAGS] [OPTIONS] <pval> [sval]

FLAGS:
        --consistent-read    Strong consistent read - to make sure retrieve the most up-to-date data. By default
                             (false), eventual consistent reads would occur.
                             https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
    -h, --help               Prints help information
    -V, --version            Prints version information

OPTIONS:
    -o, --output <output>    Switch output format [possible values: json, yaml, raw]
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
