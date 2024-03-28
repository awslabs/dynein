## dy bwrite

```
$ dy bwrite --help
dy[EXE]-bwrite 0.2.1
Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]

https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html

USAGE:
    dy[EXE] bwrite [OPTIONS]

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
        --del <dels>...      
            The item to delete in Dynein format. Each item requires at least a primary key. Multiple items can be
            specified by repeating the option. e.g. `--put '{Dynein format}' --put '{Dynein format}' --del '{Dynein
            format}'`
    -i, --input <input>      
            Input JSON file path. This input file should be BatchWriteItem input JSON syntax. For more info:
            https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
    -p, --port <port>        
            Specify the port number. This option has an effect only when `--region local` is used

        --put <puts>...      
            The item to put in Dynein format. Each item requires at least a primary key. Multiple items can be specified
            by repeating the option. e.g. `--put '{Dynein format}' --put '{Dynein format}' --del '{Dynein format}'`
    -r, --region <region>    
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
    -t, --table <table>      
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

$ dy help bwrite
dy[EXE]-bwrite 0.2.1
Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]

https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html

USAGE:
    dy[EXE] bwrite [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --del <dels>...      The item to delete in Dynein format. Each item requires at least a primary key. Multiple
                             items can be specified by repeating the option. e.g. `--put '{Dynein format}' --put
                             '{Dynein format}' --del '{Dynein format}'`
    -i, --input <input>      Input JSON file path. This input file should be BatchWriteItem input JSON syntax. For more
                             info:
                             https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
        --put <puts>...      The item to put in Dynein format. Each item requires at least a primary key. Multiple items
                             can be specified by repeating the option. e.g. `--put '{Dynein format}' --put '{Dynein
                             format}' --del '{Dynein format}'`
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

```
