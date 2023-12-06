## dy bootstrap

```
$ dy bootstrap --help
dy[EXE]-bootstrap 0.2.1
Create sample tables and load test data for bootstrapping

USAGE:
    dy[EXE] bootstrap [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -l, --list       
    -V, --version    Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -s, --sample <sample>    
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

$ dy help bootstrap
dy[EXE]-bootstrap 0.2.1
Create sample tables and load test data for bootstrapping

USAGE:
    dy[EXE] bootstrap [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -l, --list       
    -V, --version    Prints version information

OPTIONS:
    -p, --port <port>        Specify the port number. This option has an effect only when `--region local` is used
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -s, --sample <sample>    
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

```
