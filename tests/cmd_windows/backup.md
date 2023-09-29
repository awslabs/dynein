## dy backup

```
$ dy backup --help
dy[EXE]-backup 0.2.1
Take backup of a DynamoDB table using on-demand backup

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

USAGE:
    dy[EXE] backup [FLAGS] [OPTIONS]

FLAGS:
        --all-tables    
            List backups for all tables in the region

    -h, --help          
            Prints help information

    -l, --list          
            List existing DynamoDB backups

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

$ dy help backup
dy[EXE]-backup 0.2.1
Take backup of a DynamoDB table using on-demand backup

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

USAGE:
    dy[EXE] backup [FLAGS] [OPTIONS]

FLAGS:
        --all-tables    List backups for all tables in the region
    -h, --help          Prints help information
    -l, --list          List existing DynamoDB backups
    -V, --version       Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

```
