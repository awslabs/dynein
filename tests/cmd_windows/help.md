## dy help

```
$ dy --help
dynein 0.2.1
dynein is a command line tool to interact with DynamoDB tables/data using concise interface.
dynein looks for config files under $HOME/.dynein/ directory.

USAGE:
    dy[EXE] [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help                       
            Prints help information

        --shell                      
            

        --third-party-attribution    
            This option displays detailed information about third-party libraries, frameworks, and other components
            incorporated into dynein, as well as the full license texts under which they are distributed
    -V, --version                    
            Prints version information


OPTIONS:
    -p, --port <port>        
            Specify the port number. This option has an effect only when `--region local` is used

    -r, --region <region>    
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
    -t, --table <table>      
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

SUBCOMMANDS:
    admin        <sub> Admin operations such as creating/updating table or GSI
    backup       Take backup of a DynamoDB table using on-demand backup
    bootstrap    Create sample tables and load test data for bootstrapping
    bwrite       Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]
    config       <sub> Manage configuration files (config.yml and cache.yml) from command line
    del          Delete an existing item. [API: DeleteItem]
    desc         Show detailed information of a table. [API: DescribeTable]
    export       Export items from a DynamoDB table and save them as CSV/JSON file
    get          Retrieve an item by specifying primary key(s). [API: GetItem]
    help         Prints this message or the help of the given subcommand(s)
    import       Import items into a DynamoDB table from CSV/JSON file
    list         List tables in the region. [API: ListTables]
    put          Create a new item, or replace an existing item. [API: PutItem]
    query        Retrieve items that match conditions. Partition key is required. [API: Query]
    restore      Restore a DynamoDB table from backup data
    scan         Retrieve items in a table without any condition. [API: Scan]
    upd          Update an existing item. [API: UpdateItem]
    use          Switch target table context. After you use the command you don't need to specify table every time,
                 but you may overwrite the target table with --table (-t) option

$ dy help
dynein 0.2.1
dynein is a command line tool to interact with DynamoDB tables/data using concise interface.
dynein looks for config files under $HOME/.dynein/ directory.

USAGE:
    dy[EXE] [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help                       Prints help information
        --shell                      
        --third-party-attribution    This option displays detailed information about third-party libraries, frameworks,
                                     and other components incorporated into dynein, as well as the full license texts
                                     under which they are distributed
    -V, --version                    Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

SUBCOMMANDS:
    admin        <sub> Admin operations such as creating/updating table or GSI
    backup       Take backup of a DynamoDB table using on-demand backup
    bootstrap    Create sample tables and load test data for bootstrapping
    bwrite       Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]
    config       <sub> Manage configuration files (config.yml and cache.yml) from command line
    del          Delete an existing item. [API: DeleteItem]
    desc         Show detailed information of a table. [API: DescribeTable]
    export       Export items from a DynamoDB table and save them as CSV/JSON file
    get          Retrieve an item by specifying primary key(s). [API: GetItem]
    help         Prints this message or the help of the given subcommand(s)
    import       Import items into a DynamoDB table from CSV/JSON file
    list         List tables in the region. [API: ListTables]
    put          Create a new item, or replace an existing item. [API: PutItem]
    query        Retrieve items that match conditions. Partition key is required. [API: Query]
    restore      Restore a DynamoDB table from backup data
    scan         Retrieve items in a table without any condition. [API: Scan]
    upd          Update an existing item. [API: UpdateItem]
    use          Switch target table context. After you use the command you don't need to specify table every time,
                 but you may overwrite the target table with --table (-t) option

$ dy help --help
? 1
error: The subcommand '--help' wasn't recognized

USAGE:
	dy[EXE] help <subcommands>...

For more information try --help

```
