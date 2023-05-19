## dy export

```
$ dy export --help
dy-export 0.2.1
Export items from a DynamoDB table and save them as CSV/JSON file.

If you want to achieve best performance, recommendated way is to switch the table to OnDemand mode before export. (e.g.
dy admin update table your_table --mode ondemand).
 When you export items as JSON (including jsonl, json-compact), all attributes in all items will be exported.
 When you export items as CSV, on the other hand, dynein has to know which attributes are to be exported as CSV format
requires "column" - i.e. N th column should contain attribute ABC throughout a csv file.

USAGE:
    dy export [FLAGS] [OPTIONS] --output-file <output-file>

FLAGS:
    -h, --help         
            Prints help information

        --keys-only    
            [csv] Export only Primary Key(s). Effective only when --format is 'csv'

    -V, --version      
            Prints version information


OPTIONS:
    -a, --attributes <attributes>      
            [csv] Specify attributes to export, separated by commas (e.g. --attributes name,address,age). Effective only
            when --format is 'csv'.
             Note that primary key(s) are always included in results regardless of what you've passed to --attributes
    -f, --format <format>              
            Data format for export items.
             json = JSON format with newline/indent.
             jsonl = JSON Lines (http://jsonlines.org). i.e. one item per line.
             json-compact = JSON format, all items are packed in oneline.
             csv = comma-separated values with header. Use it with --keys-only or --attributes. If neither of them are
            given dynein will ask you target attributes interactively [possible values: csv, json, jsonl, json-compact]
    -o, --output-file <output-file>    
            Output target filename where dynein exports data into

    -p, --port <port>                  
            Specify the port number. This option has an effect only when `--region local` is used

    -r, --region <region>              
            The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use
            --region option in both top-level and subcommand-level
    -t, --table <table>                
            Target table of the operation. You can use --table option in both top-level and subcommand-level. You can
            store table schema locally by executing `$ dy use`, after that you need not to specify --table on every
            command

$ dy help export
dy-export 0.2.1
Export items from a DynamoDB table and save them as CSV/JSON file.

If you want to achieve best performance, recommendated way is to switch the table to OnDemand mode before export. (e.g.
dy admin update table your_table --mode ondemand).
 When you export items as JSON (including jsonl, json-compact), all attributes in all items will be exported.
 When you export items as CSV, on the other hand, dynein has to know which attributes are to be exported as CSV format
requires "column" - i.e. N th column should contain attribute ABC throughout a csv file.

USAGE:
    dy export [FLAGS] [OPTIONS] --output-file <output-file>

FLAGS:
    -h, --help         Prints help information
        --keys-only    [csv] Export only Primary Key(s). Effective only when --format is 'csv'
    -V, --version      Prints version information

OPTIONS:
    -a, --attributes <attributes>      [csv] Specify attributes to export, separated by commas (e.g. --attributes
                                       name,address,age). Effective only when --format is 'csv'.
                                        Note that primary key(s) are always included in results regardless of what
                                       you've passed to --attributes
    -f, --format <format>              Data format for export items.
                                        json = JSON format with newline/indent.
                                        jsonl = JSON Lines (http://jsonlines.org). i.e. one item per line.
                                        json-compact = JSON format, all items are packed in oneline.
                                        csv = comma-separated values with header. Use it with --keys-only or
                                       --attributes. If neither of them are given dynein will ask you target attributes
                                       interactively [possible values: csv, json, jsonl, json-compact]
    -o, --output-file <output-file>    Output target filename where dynein exports data into
    -p, --port <port>                  Specify the port number. This option has an effect only when `--region local` is
                                       used
    -r, --region <region>              The region to use (e.g. --region us-east-1). When using DynamodB Local, use
                                       `--region local`. You can use --region option in both top-level and subcommand-
                                       level
    -t, --table <table>                Target table of the operation. You can use --table option in both top-level and
                                       subcommand-level. You can store table schema locally by executing `$ dy use`,
                                       after that you need not to specify --table on every command

```
