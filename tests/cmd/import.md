## dy import

```
$ dy import --help
dy-import 0.2.1
Import items into a DynamoDB table from CSV/JSON file.

If you want to achieve best performance, recommendated way is to switch the table to OnDemand mode before import. (e.g.
dy admin update table your_table --mode ondemand).
 When you import items from a CSV file, header names are used to attributes for items. The first one or two column(s)
would be primary key(s).

USAGE:
    dy import [FLAGS] [OPTIONS] --input-file <input-file>

FLAGS:
        --enable-set-inference    
            Enable type inference for set types. This option is provided for backward compatibility

    -h, --help                    
            Prints help information

    -V, --version                 
            Prints version information


OPTIONS:
    -f, --format <format>            
            Data format for import items.
             json = JSON format with newline/indent.
             jsonl = JSON Lines (http://jsonlines.org). i.e. one item per line.
             json-compact = JSON format, all items are packed in oneline.
             csv = comma-separated values with header. Header columns are considered to be DynamoDB attributes [possible
            values: csv, json, jsonl, json-compact]
    -i, --input-file <input-file>    
            Filename contains DynamoDB items data. Specify appropriate format with --format option

    -p, --port <port>                
            Specify the port number. This option has an effect only when `--region local` is used

    -r, --region <region>            
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
    -t, --table <table>              
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

$ dy help import
dy-import 0.2.1
Import items into a DynamoDB table from CSV/JSON file.

If you want to achieve best performance, recommendated way is to switch the table to OnDemand mode before import. (e.g.
dy admin update table your_table --mode ondemand).
 When you import items from a CSV file, header names are used to attributes for items. The first one or two column(s)
would be primary key(s).

USAGE:
    dy import [FLAGS] [OPTIONS] --input-file <input-file>

FLAGS:
        --enable-set-inference    Enable type inference for set types. This option is provided for backward
                                  compatibility
    -h, --help                    Prints help information
    -V, --version                 Prints version information

OPTIONS:
    -f, --format <format>            Data format for import items.
                                      json = JSON format with newline/indent.
                                      jsonl = JSON Lines (http://jsonlines.org). i.e. one item per line.
                                      json-compact = JSON format, all items are packed in oneline.
                                      csv = comma-separated values with header. Header columns are considered to be
                                     DynamoDB attributes [possible values: csv, json, jsonl, json-compact]
    -i, --input-file <input-file>    Filename contains DynamoDB items data. Specify appropriate format with --format
                                     option
    -p, --port <port>                Specify the port number. This option has an effect only when `--region local` is
                                     used
    -r, --region <region>            The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                     `--region local`. You can use --region option in both top-level and subcommand-
                                     level
    -t, --table <table>              Target table of the operation. You can use --table option in both top-level and
                                     subcommand-level. You can store table schema locally by executing `$ dy use`, after
                                     that you need not to specify --table on every command

```
