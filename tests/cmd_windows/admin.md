## dy admin

```
$ dy admin --help
dy[EXE]-admin 0.2.1
<sub> Admin operations such as creating/updating table or GSI

USAGE:
    dy[EXE] admin [OPTIONS] <SUBCOMMAND>

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

SUBCOMMANDS:
    create    Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
    delete    Delete a DynamoDB table or GSI. [API: DeleteTable]
    desc      Show detailed information of a table. [API: DescribeTable]
    help      Prints this message or the help of the given subcommand(s)
    list      List tables in the region. [API: ListTables]
    update    Update a DynamoDB table. [API: UpdateTable etc]

$ dy help admin
dy[EXE]-admin 0.2.1
<sub> Admin operations such as creating/updating table or GSI

USAGE:
    dy[EXE] admin [OPTIONS] <SUBCOMMAND>

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

SUBCOMMANDS:
    create    Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
    delete    Delete a DynamoDB table or GSI. [API: DeleteTable]
    desc      Show detailed information of a table. [API: DescribeTable]
    help      Prints this message or the help of the given subcommand(s)
    list      List tables in the region. [API: ListTables]
    update    Update a DynamoDB table. [API: UpdateTable etc]

$ dy admin help
dy[EXE]-admin 0.2.1
<sub> Admin operations such as creating/updating table or GSI

USAGE:
    dy[EXE] admin [OPTIONS] <SUBCOMMAND>

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

SUBCOMMANDS:
    create    Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
    delete    Delete a DynamoDB table or GSI. [API: DeleteTable]
    desc      Show detailed information of a table. [API: DescribeTable]
    help      Prints this message or the help of the given subcommand(s)
    list      List tables in the region. [API: ListTables]
    update    Update a DynamoDB table. [API: UpdateTable etc]

$ dy admin create --help
dy[EXE]-admin-create 0.2.1
Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]

USAGE:
    dy[EXE] admin create [OPTIONS] <SUBCOMMAND>

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

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    index    Create new GSI (global secondary index) for a table with given primary key(s). [API: UpdateTable]
    table    Create new DynamoDB table with given primary key(s). [API: CreateTable]

$ dy admin create index --help
dy[EXE]-admin-create-index 0.2.1
Create new GSI (global secondary index) for a table with given primary key(s). [API: UpdateTable]

USAGE:
    dy[EXE] admin create index [OPTIONS] <index-name> --keys <keys>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -k, --keys <keys>...     (requried) Primary key(s) of the index. Key name followed by comma and data type (S/N/B).
                             e.g. for Partition key only table: `--keys myPk,S`, and for Partition and Sort key table
                             `--keys myPk,S mySk,N`
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <index-name>    index name to create

$ dy admin create table --help
dy[EXE]-admin-create-table 0.2.1
Create new DynamoDB table with given primary key(s). [API: CreateTable]

USAGE:
    dy[EXE] admin create table [OPTIONS] <new-table-name> --keys <keys>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -k, --keys <keys>...     (requried) Primary key(s) of the table. Key name followed by comma and data type (S/N/B).
                             e.g. for Partition key only table: `--keys myPk,S`, and for Partition and Sort key table
                             `--keys myPk,S mySk,N`
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <new-table-name>    table name to create

$ dy admin delete --help
dy[EXE]-admin-delete 0.2.1
Delete a DynamoDB table or GSI. [API: DeleteTable]

USAGE:
    dy[EXE] admin delete [OPTIONS] <SUBCOMMAND>

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

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    table    Delete a DynamoDB table

$ dy admin delete table --help
dy[EXE]-admin-delete-table 0.2.1
Delete a DynamoDB table

USAGE:
    dy[EXE] admin delete table [FLAGS] [OPTIONS] <table-name-to-delete>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
        --yes        Skip interactive confirmation before deleting a table

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <table-name-to-delete>    table name to delete

$ dy admin desc --help
dy[EXE]-admin-desc 0.2.1
Show detailed information of a table. [API: DescribeTable]

USAGE:
    dy[EXE] admin desc [FLAGS] [OPTIONS] [target-table-to-desc]

FLAGS:
        --all-tables    Show details of all tables in the region
    -h, --help          Prints help information
    -V, --version       Prints version information

OPTIONS:
    -o, --output <output>    Switch output format [possible values: yaml]
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

ARGS:
    <target-table-to-desc>    Target table name. Optionally you may specify the target table by --table (-t) option

$ dy admin list --help
dy[EXE]-admin-list 0.2.1
List tables in the region. [API: ListTables]

USAGE:
    dy[EXE] admin list [FLAGS] [OPTIONS]

FLAGS:
        --all-regions    List DynamoDB tables in all available regions
    -h, --help           Prints help information
    -V, --version        Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

$ dy admin update --help
dy[EXE]-admin-update 0.2.1
Update a DynamoDB table. [API: UpdateTable etc]

USAGE:
    dy[EXE] admin update [OPTIONS] <SUBCOMMAND>

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

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    table    Update a DynamoDB table

$ dy admin update table --help
dy[EXE]-admin-update-table 0.2.1
Update a DynamoDB table

USAGE:
    dy[EXE] admin update table [OPTIONS] <table-name-to-update>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m, --mode <mode>        DynamoDB capacity mode. Availablle values: [provisioned, ondemand]. When you switch from
                             OnDemand to Provisioned mode, you can pass WCU and RCU as well (NOTE: default capacity unit
                             for Provisioned mode is 5) [possible values: provisioned, ondemand]
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
        --rcu <rcu>          RCU (read capacity units) for the table. Acceptable only on Provisioned mode
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command
        --wcu <wcu>          WCU (write capacity units) for the table. Acceptable only on Provisioned mode

ARGS:
    <table-name-to-update>    table name to update

```
