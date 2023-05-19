## dy restore

```
$ dy restore --help
dy-restore 0.2.1
Restore a DynamoDB table from backup data

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

USAGE:
    dy restore [OPTIONS]

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
    -b, --backup-name <backup-name>      
            Specify backup file. If not specified you can select it interactively

    -p, --port <port>                    
            Specify the port number. This option has an effect only when `--region local` is used

    -r, --region <region>                
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
        --restore-name <restore-name>    
            Name of the newly restored table. If not specified, default naming rule "<source-table-
            name>-restore-<timestamp>" would be used
    -t, --table <table>                  
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

$ dy help restore
dy-restore 0.2.1
Restore a DynamoDB table from backup data

For more details: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/BackupRestore.html

USAGE:
    dy restore [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --backup-name <backup-name>      Specify backup file. If not specified you can select it interactively
    -p, --port <port>                    Specify the port number. This option has an effect only when `--region local`
                                         is used
    -r, --region <region>                The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                         `--region local`. You can use --region option in both top-level and subcommand-
                                         level
        --restore-name <restore-name>    Name of the newly restored table. If not specified, default naming rule
                                         "<source-table-name>-restore-<timestamp>" would be used
    -t, --table <table>                  Target table of the operation. You can use --table option in both top-level and
                                         subcommand-level. You can store table schema locally by executing `$ dy use`,
                                         after that you need not to specify --table on every command

```
