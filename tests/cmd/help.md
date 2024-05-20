## dy help

```
$ dy --help
dynein is a command line tool to interact with DynamoDB tables/data using concise interface.
dynein looks for config files under $HOME/.dynein/ directory.

Usage: dy [OPTIONS] [COMMAND]

Commands:
  admin      <sub> Admin operations such as creating/updating table or GSI
  list       List tables in the region. [API: ListTables]
  desc       Show detailed information of a table. [API: DescribeTable]
  scan       Retrieve items in a table without any condition. [API: Scan]
  get        Retrieve an item by specifying primary key(s). [API: GetItem]
  query      Retrieve items that match conditions. Partition key is required. [API: Query]
  put        Create a new item, or replace an existing item. [API: PutItem]
  del        Delete an existing item. [API: DeleteItem]
  upd        Update an existing item. [API: UpdateItem]
  bwrite     Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]
  use        Switch target table context. After you use the command you don't need to specify table every time, but you may overwrite the target table with --table (-t) option.
  config     <sub> Manage configuration files (config.yml and cache.yml) from command line
  bootstrap  Create sample tables and load test data for bootstrapping
  export     Export items from a DynamoDB table and save them as CSV/JSON file.
  import     Import items into a DynamoDB table from CSV/JSON file.
  backup     Take backup of a DynamoDB table using on-demand backup
  restore    Restore a DynamoDB table from backup data
  help       Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>          The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                                 You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>              Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>            Target table of the operation. You can use --table option in both top-level and subcommand-level.
                                 You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
      --shell                    
      --third-party-attribution  This option displays detailed information about third-party libraries, frameworks, and other components incorporated into dynein, as well as the full license texts under which they are distributed
  -h, --help                     Print help
  -V, --version                  Print version

$ dy help
dynein is a command line tool to interact with DynamoDB tables/data using concise interface.
dynein looks for config files under $HOME/.dynein/ directory.

Usage: dy [OPTIONS] [COMMAND]

Commands:
  admin      <sub> Admin operations such as creating/updating table or GSI
  list       List tables in the region. [API: ListTables]
  desc       Show detailed information of a table. [API: DescribeTable]
  scan       Retrieve items in a table without any condition. [API: Scan]
  get        Retrieve an item by specifying primary key(s). [API: GetItem]
  query      Retrieve items that match conditions. Partition key is required. [API: Query]
  put        Create a new item, or replace an existing item. [API: PutItem]
  del        Delete an existing item. [API: DeleteItem]
  upd        Update an existing item. [API: UpdateItem]
  bwrite     Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]
  use        Switch target table context. After you use the command you don't need to specify table every time, but you may overwrite the target table with --table (-t) option.
  config     <sub> Manage configuration files (config.yml and cache.yml) from command line
  bootstrap  Create sample tables and load test data for bootstrapping
  export     Export items from a DynamoDB table and save them as CSV/JSON file.
  import     Import items into a DynamoDB table from CSV/JSON file.
  backup     Take backup of a DynamoDB table using on-demand backup
  restore    Restore a DynamoDB table from backup data
  help       Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>          The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                                 You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>              Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>            Target table of the operation. You can use --table option in both top-level and subcommand-level.
                                 You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
      --shell                    
      --third-party-attribution  This option displays detailed information about third-party libraries, frameworks, and other components incorporated into dynein, as well as the full license texts under which they are distributed
  -h, --help                     Print help
  -V, --version                  Print version

$ dy help --help
? 2
error: unrecognized subcommand '--help'

Usage: dy [OPTIONS] [COMMAND]

For more information, try '--help'.

```
