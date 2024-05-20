## dy query

```
$ dy query --help
Retrieve items that match conditions. Partition key is required. [API: Query]

Usage: dy[EXE] query [OPTIONS] <PVAL>

Arguments:
  <PVAL>
          Target Partition Key

Options:
  -s, --sort-key <SORT_KEY_EXPRESSION>
          Additional Sort Key condition which will be converted to KeyConditionExpression.
          Valid syntax: ['= 12', '> 12', '>= 12', '< 12', '<= 12', 'between 10 and 99', 'begins_with myVal"]

      --consistent-read
          Strong consistent read - to make sure retrieve the most up-to-date data. By default (false), eventual consistent reads would occur.
          https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html

  -i, --index <INDEX>
          Read data from index instead of base table.

  -l, --limit <LIMIT>
          Limit the number of items to return. By default, the number of items is determined by DynamoDB.

  -a, --attributes <ATTRIBUTES>
          Attributes to show, separated by commas, which is mapped to ProjectionExpression (e.g. --attributes name,address,age).
          Note that primary key(s) are always included in results regardless of what you've passed to --attributes.

      --keys-only
          Show only Primary Key(s).

  -d, --descending
          Results of query are always sorted by the sort key value. By default, the sort order is ascending.
          Specify --descending to traverse descending order.

      --strict
          Specify the strict mode for parsing query conditions. By default, the non-strict mode is used unless specified on the config file. You cannot combine with --non-strict option.
          
          In strict mode, you will experience an error if the provided value does not match the table schema.

      --non-strict
          Specify the non-strict mode for parsing query conditions. By default, the non-strict mode is used unless specified on the config file. You cannot combine with --strict option.
          
          In non-strict mode, dynein tries to infer the intention of the provided expression as much as possible.

  -o, --output <OUTPUT>
          Switch output format.
          
          [possible values: table, json, raw]

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

$ dy help query
Retrieve items that match conditions. Partition key is required. [API: Query]

Usage: dy[EXE] query [OPTIONS] <PVAL>

Arguments:
  <PVAL>
          Target Partition Key

Options:
  -s, --sort-key <SORT_KEY_EXPRESSION>
          Additional Sort Key condition which will be converted to KeyConditionExpression.
          Valid syntax: ['= 12', '> 12', '>= 12', '< 12', '<= 12', 'between 10 and 99', 'begins_with myVal"]

      --consistent-read
          Strong consistent read - to make sure retrieve the most up-to-date data. By default (false), eventual consistent reads would occur.
          https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html

  -i, --index <INDEX>
          Read data from index instead of base table.

  -l, --limit <LIMIT>
          Limit the number of items to return. By default, the number of items is determined by DynamoDB.

  -a, --attributes <ATTRIBUTES>
          Attributes to show, separated by commas, which is mapped to ProjectionExpression (e.g. --attributes name,address,age).
          Note that primary key(s) are always included in results regardless of what you've passed to --attributes.

      --keys-only
          Show only Primary Key(s).

  -d, --descending
          Results of query are always sorted by the sort key value. By default, the sort order is ascending.
          Specify --descending to traverse descending order.

      --strict
          Specify the strict mode for parsing query conditions. By default, the non-strict mode is used unless specified on the config file. You cannot combine with --non-strict option.
          
          In strict mode, you will experience an error if the provided value does not match the table schema.

      --non-strict
          Specify the non-strict mode for parsing query conditions. By default, the non-strict mode is used unless specified on the config file. You cannot combine with --strict option.
          
          In non-strict mode, dynein tries to infer the intention of the provided expression as much as possible.

  -o, --output <OUTPUT>
          Switch output format.
          
          [possible values: table, json, raw]

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
