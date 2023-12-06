## dy upd

```
$ dy upd --help
dy[EXE]-upd 0.2.1
Update an existing item. [API: UpdateItem]

This command accepts --set or --remove option and generates DynamoDB's UpdateExpression that is passed to UpdateItem
API. Note that modifying primary key(s) means item replacement in DynamoDB, so updating pk/sk is not allowed in API
level. For more information: https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html
https://docs.amazonaws.cn/en_us/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html

USAGE:
    dy[EXE] upd [OPTIONS] <pval> [sval]

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
        --atomic-counter <atomic-counter>    
            Increment a Number attribute by 1. e.g. `dy update <keys> --atomic-counter sitePv`

    -p, --port <port>                        
            Specify the port number. This option has an effect only when `--region local` is used

    -r, --region <region>                    
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
        --remove <remove>                    
            REMOVE action to remove attribute(s) from an item. --remove cannot be used with --set. e.g. --remove
            'Category, Rank'
        --set <set>                          
            SET action to modify or add attribute(s) of an item. --set cannot be used with --remove. e.g. --set 'name =
            Alice', --set 'Price = Price + 100', or --set 'Replies = 2, Closed = true, LastUpdated = "2020-02-
            22T18:10:57Z"'
    -t, --table <table>                      
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

ARGS:
    <pval>    
            Partition Key of the target item

    <sval>    
            Sort Key of the target item (if any)


$ dy help upd
dy[EXE]-upd 0.2.1
Update an existing item. [API: UpdateItem]

This command accepts --set or --remove option and generates DynamoDB's UpdateExpression that is passed to UpdateItem
API. Note that modifying primary key(s) means item replacement in DynamoDB, so updating pk/sk is not allowed in API
level. For more information: https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html
https://docs.amazonaws.cn/en_us/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html

USAGE:
    dy[EXE] upd [OPTIONS] <pval> [sval]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --atomic-counter <atomic-counter>    Increment a Number attribute by 1. e.g. `dy update <keys> --atomic-counter
                                             sitePv`
    -p, --port <port>                        Specify the port number. This option has an effect only when `--region
                                             local` is used
    -r, --region <region>                    The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                             `--region local`. You can use --region option in both top-level and
                                             subcommand-level
        --remove <remove>                    REMOVE action to remove attribute(s) from an item. --remove cannot be used
                                             with --set. e.g. --remove 'Category, Rank'
        --set <set>                          SET action to modify or add attribute(s) of an item. --set cannot be used
                                             with --remove. e.g. --set 'name = Alice', --set 'Price = Price + 100', or
                                             --set 'Replies = 2, Closed = true, LastUpdated = "2020-02-22T18:10:57Z"'
    -t, --table <table>                      Target table of the operation. You can use --table option in both top-level
                                             and subcommand-level. You can store table schema locally by executing `$ dy
                                             use`, after that you need not to specify --table on every command

ARGS:
    <pval>    Partition Key of the target item
    <sval>    Sort Key of the target item (if any)

```
