## dy scan

```
$ dy scan --help
dy[EXE]-scan 0.2.1
Retrieve items in a table without any condition. [API: Scan]

USAGE:
    dy[EXE] scan [FLAGS] [OPTIONS]

FLAGS:
        --consistent-read    Strong consistent read - to make sure retrieve the most up-to-date data. By default
                             (false), eventual consistent reads would occur.
                             https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
    -h, --help               Prints help information
        --keys-only          Show only Primary Key(s)
    -V, --version            Prints version information

OPTIONS:
    -a, --attributes <attributes>    Attributes to show, separated by commas, which is mapped to ProjectionExpression
                                     (e.g. --attributes name,address,age). Note that primary key(s) are always included
                                     in results regardless of what you've passed to --attributes
    -i, --index <index>              Read data from index instead of base table
    -l, --limit <limit>              Limit number of items to return [default: 100]
    -o, --output <output>            Switch output format [possible values: table, json, raw]
    -p, --port <port>                Specify the port number. This option has an effect only when `--region local` is
                                     used
    -r, --region <region>            The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                     `--region local`. You can use --region option in both top-level and subcommand-
                                     level
    -t, --table <table>              Target table of the operation. You can use --table option in both top-level and
                                     subcommand-level. You can store table schema locally by executing `$ dy use`, after
                                     that you need not to specify --table on every command

$ dy help scan
dy[EXE]-scan 0.2.1
Retrieve items in a table without any condition. [API: Scan]

USAGE:
    dy[EXE] scan [FLAGS] [OPTIONS]

FLAGS:
        --consistent-read    Strong consistent read - to make sure retrieve the most up-to-date data. By default
                             (false), eventual consistent reads would occur.
                             https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
    -h, --help               Prints help information
        --keys-only          Show only Primary Key(s)
    -V, --version            Prints version information

OPTIONS:
    -a, --attributes <attributes>    Attributes to show, separated by commas, which is mapped to ProjectionExpression
                                     (e.g. --attributes name,address,age). Note that primary key(s) are always included
                                     in results regardless of what you've passed to --attributes
    -i, --index <index>              Read data from index instead of base table
    -l, --limit <limit>              Limit number of items to return [default: 100]
    -o, --output <output>            Switch output format [possible values: table, json, raw]
    -p, --port <port>                Specify the port number. This option has an effect only when `--region local` is
                                     used
    -r, --region <region>            The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                     `--region local`. You can use --region option in both top-level and subcommand-
                                     level
    -t, --table <table>              Target table of the operation. You can use --table option in both top-level and
                                     subcommand-level. You can store table schema locally by executing `$ dy use`, after
                                     that you need not to specify --table on every command

```
