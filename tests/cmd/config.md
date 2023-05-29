## dy config

```
$ dy config --help
<sub> Manage configuration files (config.yml and cache.yml) from command line

Usage: dy config [OPTIONS] <COMMAND>

Commands:
  dump   Show all configuration in config (config.yml) and cache (cache.yml) files
  clear  Reset all dynein configuration in the `~/.dynein/` directory. This command initializes dynein related files only and won't remove your data stored in DynamoDB tables
  help   Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help             Print help

$ dy help config
<sub> Manage configuration files (config.yml and cache.yml) from command line

Usage: dy config [OPTIONS] <COMMAND>

Commands:
  dump   Show all configuration in config (config.yml) and cache (cache.yml) files
  clear  Reset all dynein configuration in the `~/.dynein/` directory. This command initializes dynein related files only and won't remove your data stored in DynamoDB tables
  help   Print this message or the help of the given subcommand(s)

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help             Print help

$ dy config clear --help
Reset all dynein configuration in the `~/.dynein/` directory. This command initializes dynein related files only and won't remove your data stored in DynamoDB tables

Usage: dy config clear [OPTIONS]

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help             Print help

$ dy config dump --help
Show all configuration in config (config.yml) and cache (cache.yml) files

Usage: dy config dump [OPTIONS]

Options:
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`. You can use --region option in both top-level and subcommand-level
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level. You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command
  -h, --help             Print help

```
