## dy config

```
$ dy config --help
dy[EXE]-config 0.2.1
<sub> Manage configuration files (config.yml and cache.yml) from command line

USAGE:
    dy[EXE] config [OPTIONS] <SUBCOMMAND>

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
    clear    Reset all dynein configuration in the `~/.dynein/` directory. This command initializes dynein related
             files only and won't remove your data stored in DynamoDB tables
    dump     Show all configuration in config (config.yml) and cache (cache.yml) files
    help     Prints this message or the help of the given subcommand(s)

$ dy help config
dy[EXE]-config 0.2.1
<sub> Manage configuration files (config.yml and cache.yml) from command line

USAGE:
    dy[EXE] config [OPTIONS] <SUBCOMMAND>

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
    clear    Reset all dynein configuration in the `~/.dynein/` directory. This command initializes dynein related
             files only and won't remove your data stored in DynamoDB tables
    dump     Show all configuration in config (config.yml) and cache (cache.yml) files
    help     Prints this message or the help of the given subcommand(s)

$ dy config clear --help
dy[EXE]-config-clear 0.2.1
Reset all dynein configuration in the `~/.dynein/` directory. This command initializes dynein related files only and
won't remove your data stored in DynamoDB tables

USAGE:
    dy[EXE] config clear [OPTIONS]

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

$ dy config dump --help
dy[EXE]-config-dump 0.2.1
Show all configuration in config (config.yml) and cache (cache.yml) files

USAGE:
    dy[EXE] config dump [OPTIONS]

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

```
