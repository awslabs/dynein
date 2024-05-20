## dy admin

```
$ dy admin --help
<sub> Admin operations such as creating/updating table or GSI

Usage: dy admin [OPTIONS] <COMMAND>

Commands:
  list    List tables in the region. [API: ListTables]
  desc    Show detailed information of a table. [API: DescribeTable]
  create  Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
  update  Update a DynamoDB table. [API: UpdateTable etc]
  delete  Delete a DynamoDB table or GSI. [API: DeleteTable]
  help    Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy help admin
<sub> Admin operations such as creating/updating table or GSI

Usage: dy admin [OPTIONS] <COMMAND>

Commands:
  list    List tables in the region. [API: ListTables]
  desc    Show detailed information of a table. [API: DescribeTable]
  create  Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
  update  Update a DynamoDB table. [API: UpdateTable etc]
  delete  Delete a DynamoDB table or GSI. [API: DeleteTable]
  help    Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin help
<sub> Admin operations such as creating/updating table or GSI

Usage: dy admin [OPTIONS] <COMMAND>

Commands:
  list    List tables in the region. [API: ListTables]
  desc    Show detailed information of a table. [API: DescribeTable]
  create  Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
  update  Update a DynamoDB table. [API: UpdateTable etc]
  delete  Delete a DynamoDB table or GSI. [API: DeleteTable]
  help    Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin create --help
Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]

Usage: dy admin create [OPTIONS] <COMMAND>

Commands:
  table  Create new DynamoDB table with given primary key(s). [API: CreateTable]
  index  Create new GSI (global secondary index) for a table with given primary key(s). [API: UpdateTable]
  help   Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin create index --help
Create new GSI (global secondary index) for a table with given primary key(s). [API: UpdateTable]

Usage: dy admin create index [OPTIONS] --keys <KEYS>... <INDEX_NAME>

Arguments:
  <INDEX_NAME>  index name to create

Options:
  -k, --keys <KEYS>...   (requried) Primary key(s) of the index. Key name followed by comma and data type (S/N/B).
                         e.g. for Partition key only table: `--keys myPk,S`, and for Partition and Sort key table `--keys myPk,S mySk,N`
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin create table --help
Create new DynamoDB table with given primary key(s). [API: CreateTable]

Usage: dy admin create table [OPTIONS] --keys <KEYS>... <NEW_TABLE_NAME>

Arguments:
  <NEW_TABLE_NAME>  table name to create

Options:
  -k, --keys <KEYS>...   (requried) Primary key(s) of the table. Key name followed by comma and data type (S/N/B).
                         e.g. for Partition key only table: `--keys myPk,S`, and for Partition and Sort key table `--keys myPk,S mySk,N`
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin delete --help
Delete a DynamoDB table or GSI. [API: DeleteTable]

Usage: dy admin delete [OPTIONS] <COMMAND>

Commands:
  table  Delete a DynamoDB table.
  help   Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin delete table --help
Delete a DynamoDB table.

Usage: dy admin delete table [OPTIONS] <TABLE_NAME_TO_DELETE>

Arguments:
  <TABLE_NAME_TO_DELETE>  table name to delete

Options:
      --yes              Skip interactive confirmation before deleting a table.
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin desc --help
Show detailed information of a table. [API: DescribeTable]

Usage: dy admin desc [OPTIONS] [TARGET_TABLE_TO_DESC]

Arguments:
  [TARGET_TABLE_TO_DESC]  Target table name. Optionally you may specify the target table by --table (-t) option

Options:
      --all-tables       Show details of all tables in the region
  -o, --output <OUTPUT>  Switch output format. [possible values: yaml]
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin list --help
List tables in the region. [API: ListTables]

Usage: dy admin list [OPTIONS]

Options:
      --all-regions      List DynamoDB tables in all available regions
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin update --help
Update a DynamoDB table. [API: UpdateTable etc]

Usage: dy admin update [OPTIONS] <COMMAND>

Commands:
  table  Update a DynamoDB table.
  help   Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy admin update table --help
Update a DynamoDB table.

Usage: dy admin update table [OPTIONS] <TABLE_NAME_TO_UPDATE>

Arguments:
  <TABLE_NAME_TO_UPDATE>  table name to update

Options:
  -m, --mode <MODE>      DynamoDB capacity mode. Availablle values: [provisioned, ondemand].
                         When you switch from OnDemand to Provisioned mode, you can pass WCU and RCU as well (NOTE: default capacity unit for Provisioned mode is 5). [possible values: provisioned, ondemand]
      --wcu <WCU>        WCU (write capacity units) for the table. Acceptable only on Provisioned mode.
      --rcu <RCU>        RCU (read capacity units) for the table. Acceptable only on Provisioned mode.
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

```
