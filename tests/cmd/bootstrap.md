## dy bootstrap

```
$ dy bootstrap --help
Create sample tables and load test data for bootstrapping

Usage: dy bootstrap [OPTIONS]

Options:
  -l, --list             
  -s, --sample <SAMPLE>  
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

$ dy help bootstrap
Create sample tables and load test data for bootstrapping

Usage: dy bootstrap [OPTIONS]

Options:
  -l, --list             
  -s, --sample <SAMPLE>  
  -r, --region <REGION>  The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region local`.
                         You can use --region option in both top-level and subcommand-level.
  -p, --port <PORT>      Specify the port number. This option has an effect only when `--region local` is used.
  -t, --table <TABLE>    Target table of the operation. You can use --table option in both top-level and subcommand-level.
                         You can store table schema locally by executing `$ dy use`, after that you need not to specify --table on every command.
  -h, --help             Print help

```
