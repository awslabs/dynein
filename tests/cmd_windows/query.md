## dy query

```
$ dy query --help
dy[EXE]-query 0.2.1
Retrieve items that match conditions. Partition key is required. [API: Query]

USAGE:
    dy[EXE] query [FLAGS] [OPTIONS] <pval>

FLAGS:
        --consistent-read    Strong consistent read - to make sure retrieve the most up-to-date data. By default
                             (false), eventual consistent reads would occur.
                             https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
    -d, --descending         Results of query are always sorted by the sort key value. By default, the sort order is
                             ascending. Specify --descending to traverse descending order
    -h, --help               Prints help information
        --keys-only          Show only Primary Key(s)
    -V, --version            Prints version information

OPTIONS:
    -a, --attributes <attributes>           Attributes to show, separated by commas, which is mapped to
                                            ProjectionExpression (e.g. --attributes name,address,age). Note that primary
                                            key(s) are always included in results regardless of what you've passed to
                                            --attributes
    -i, --index <index>                     Read data from index instead of base table
    -l, --limit <limit>                     Limit the number of items to return. By default, the number of items is
                                            determined by DynamoDB
    -o, --output <output>                   Switch output format [possible values: table, json, raw]
    -p, --port <port>                       Specify the port number. This option has an effect only when `--region
                                            local` is used
    -r, --region <region>                   The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                            `--region local`. You can use --region option in both top-level and
                                            subcommand-level
    -s, --sort-key <sort-key-expression>    Additional Sort Key condition which will be converted to
                                            KeyConditionExpression. Valid syntax: ['= 12', '> 12', '>= 12', '< 12', '<=
                                            12', 'between 10 and 99', 'begins_with myVal"]
    -t, --table <table>                     Target table of the operation. You can use --table option in both top-level
                                            and subcommand-level. You can store table schema locally by executing `$ dy
                                            use`, after that you need not to specify --table on every command

ARGS:
    <pval>    Target Partition Key

$ dy help query
dy[EXE]-query 0.2.1
Retrieve items that match conditions. Partition key is required. [API: Query]

USAGE:
    dy[EXE] query [FLAGS] [OPTIONS] <pval>

FLAGS:
        --consistent-read    Strong consistent read - to make sure retrieve the most up-to-date data. By default
                             (false), eventual consistent reads would occur.
                             https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
    -d, --descending         Results of query are always sorted by the sort key value. By default, the sort order is
                             ascending. Specify --descending to traverse descending order
    -h, --help               Prints help information
        --keys-only          Show only Primary Key(s)
    -V, --version            Prints version information

OPTIONS:
    -a, --attributes <attributes>           Attributes to show, separated by commas, which is mapped to
                                            ProjectionExpression (e.g. --attributes name,address,age). Note that primary
                                            key(s) are always included in results regardless of what you've passed to
                                            --attributes
    -i, --index <index>                     Read data from index instead of base table
    -l, --limit <limit>                     Limit the number of items to return. By default, the number of items is
                                            determined by DynamoDB
    -o, --output <output>                   Switch output format [possible values: table, json, raw]
    -p, --port <port>                       Specify the port number. This option has an effect only when `--region
                                            local` is used
    -r, --region <region>                   The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                            `--region local`. You can use --region option in both top-level and
                                            subcommand-level
    -s, --sort-key <sort-key-expression>    Additional Sort Key condition which will be converted to
                                            KeyConditionExpression. Valid syntax: ['= 12', '> 12', '>= 12', '< 12', '<=
                                            12', 'between 10 and 99', 'begins_with myVal"]
    -t, --table <table>                     Target table of the operation. You can use --table option in both top-level
                                            and subcommand-level. You can store table schema locally by executing `$ dy
                                            use`, after that you need not to specify --table on every command

ARGS:
    <pval>    Target Partition Key

```
