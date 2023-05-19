## dy bwrite

```
$ dy bwrite --help
dy-bwrite 0.2.1
Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]

https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html

USAGE:
    dy bwrite [OPTIONS] --input <input>

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
    -i, --input <input>      
            Input JSON file path. This input file should be BatchWriteItem input JSON syntax. For more info:
            https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
    -p, --port <port>        
            Specify the port number. This option has an effect only when `--region local` is used

    -r, --region <region>    
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
    -t, --table <table>      
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

$ dy help bwrite
dy-bwrite 0.2.1
Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]

https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html

USAGE:
    dy bwrite [OPTIONS] --input <input>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --input <input>      Input JSON file path. This input file should be BatchWriteItem input JSON syntax. For more
                             info:
                             https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

```
