## dy scan

```
$ dy scan --help
Retrieve items in a table without any condition. [API: Scan]

Usage: dy scan [OPTIONS]

Options:
  -l, --limit <LIMIT>            Limit number of items to return [default: 100]
  -a, --attributes <ATTRIBUTES>  Attributes to show, separated by commas, which is mapped to ProjectionExpression (e.g. --attributes name,address,age). Note that primary key(s) are always included in results regardless of what you've passed to --attributes
      --consistent-read          Strong consistent read - to make sure retrieve the most up-to-date data. By default (false), eventual consistent reads would occur. https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
      --keys-only                Show only Primary Key(s)
  -i, --index <INDEX>            Read data from index instead of base table
  -o, --output <OUTPUT>          Switch output format [possible values: table, json, raw]
  -r, --region <REGION>          The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>              Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help                     Print help

$ dy help scan
Retrieve items in a table without any condition. [API: Scan]

Usage: dy scan [OPTIONS]

Options:
  -l, --limit <LIMIT>            Limit number of items to return [default: 100]
  -a, --attributes <ATTRIBUTES>  Attributes to show, separated by commas, which is mapped to ProjectionExpression (e.g. --attributes name,address,age). Note that primary key(s) are always included in results regardless of what you've passed to --attributes
      --consistent-read          Strong consistent read - to make sure retrieve the most up-to-date data. By default (false), eventual consistent reads would occur. https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html
      --keys-only                Show only Primary Key(s)
  -i, --index <INDEX>            Read data from index instead of base table
  -o, --output <OUTPUT>          Switch output format [possible values: table, json, raw]
  -r, --region <REGION>          The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>              Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help                     Print help

```
