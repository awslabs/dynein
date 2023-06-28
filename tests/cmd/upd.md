## dy upd

```
$ dy upd --help
Update an existing item. [API: UpdateItem]

This command accepts --set or --remove option and generates DynamoDB's UpdateExpression that is passed to UpdateItem API.
Note that modifying primary key(s) means item replacement in DynamoDB, so updating pk/sk is not allowed in API level.
For more information:
https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html
https://docs.amazonaws.cn/en_us/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html

Usage: dy upd [OPTIONS] <PVAL> [SVAL]

Arguments:
  <PVAL>
          Partition Key of the target item

  [SVAL]
          Sort Key of the target item (if any)

Options:
      --set <SET>
          SET action to modify or add attribute(s) of an item. --set cannot be used with --remove.
          e.g. --set 'name = Alice', --set 'Price = Price + 100', or --set 'Replies = 2, Closed = true, LastUpdated = "2020-02-22T18:10:57Z"'

      --remove <REMOVE>
          REMOVE action to remove attribute(s) from an item. --remove cannot be used with --set.
          e.g. --remove 'Category, Rank'

      --atomic-counter <ATOMIC_COUNTER>
          Increment a Number attribute by 1. e.g. `dy update <keys> --atomic-counter sitePv`.

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

$ dy help upd
Update an existing item. [API: UpdateItem]

This command accepts --set or --remove option and generates DynamoDB's UpdateExpression that is passed to UpdateItem API.
Note that modifying primary key(s) means item replacement in DynamoDB, so updating pk/sk is not allowed in API level.
For more information:
https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html
https://docs.amazonaws.cn/en_us/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html

Usage: dy upd [OPTIONS] <PVAL> [SVAL]

Arguments:
  <PVAL>
          Partition Key of the target item

  [SVAL]
          Sort Key of the target item (if any)

Options:
      --set <SET>
          SET action to modify or add attribute(s) of an item. --set cannot be used with --remove.
          e.g. --set 'name = Alice', --set 'Price = Price + 100', or --set 'Replies = 2, Closed = true, LastUpdated = "2020-02-22T18:10:57Z"'

      --remove <REMOVE>
          REMOVE action to remove attribute(s) from an item. --remove cannot be used with --set.
          e.g. --remove 'Category, Rank'

      --atomic-counter <ATOMIC_COUNTER>
          Increment a Number attribute by 1. e.g. `dy update <keys> --atomic-counter sitePv`.

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
