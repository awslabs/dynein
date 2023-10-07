dynein - DynamoDB CLI
========================================

**dynein** /daɪ.nɪn/ is a command line interface for [Amazon DynamoDB](https://aws.amazon.com/dynamodb/) written in Rust. dynein is designed to make it simple to interact with DynamoDB tables/items from terminal.

<!-- TOC -->

- [Why use dynein?](#why-use-dynein)
    - [Less Typing](#less-typing)
    - [Quick Start](#quick-start)
    - [For day-to-day tasks](#for-day-to-day-tasks)
- [Installation](#installation)
    - [Method 1. Download binaries](#method-1-download-binaries)
    - [Method 2. Homebrew (MacOS)](#method-2-homebrew-macos)
    - [Method 3. Building from source](#method-3-building-from-source)
- [How to Use](#how-to-use)
    - [Prerequisites - AWS Credentials](#prerequisites---aws-credentials)
    - [Commands overview](#commands-overview)
    - [Bootstrapping sample DynamoDB tables](#bootstrapping-sample-dynamodb-tables)
    - [Working with DynamoDB tables](#working-with-dynamodb-tables)
        - [Infrastracture as Code - enpowered by CloudFormation](#infrastracture-as-code---enpowered-by-cloudformation)
        - [`dy use` and `dy config` to switch/manage context](#dy-use-and-dy-config-to-switchmanage-context)
    - [Working with DynamoDB items](#working-with-dynamodb-items)
        - [Read](#read)
            - [`dy scan`](#dy-scan)
            - [`dy get`](#dy-get)
            - [`dy query`](#dy-query)
        - [Write](#write)
            - [`dy put`](#dy-put)
            - [`dy upd`](#dy-upd)
            - [`dy del`](#dy-del)
    - [Working with Indexes](#working-with-indexes)
    - [Import/Export for DynamoDB items](#importexport-for-dynamodb-items)
        - [`dy export`](#dy-export)
        - [`dy import`](#dy-import)
    - [Using DynamoDB Local with `--region local` option](#using-dynamodb-local-with---region-local-option)
- [Misc](#misc)
    - [Development](#development)
    - [Asides](#asides)
    - [Troubleshooting](#troubleshooting)
    - [Ideas for future works](#ideas-for-future-works)

<!-- /TOC -->


# Why use dynein?

## Less Typing

- Auto completion for table/keyDefinitions enables using DynamoDB with minimum arguments. e.g. to get an item: `dy get abc`
- Switching table context by RDBMS-ish "use".
- Prefer standard JSON ( `{"id": 123}` ) over DynamoDB JSON ( `{"id": {"N": "123"}}` ).

## Quick Start

- [Bootstrap command](#bootstrapping-sample-dynamodb-tables) enables you to launch sample table with sample data sets.
- Supports DynamoDB Local and you can test DyanmoDB at no charge.

## For day-to-day tasks

- Import/Export by single command: export DynamoDB items to CSV/JSON files and conversely, import them into tables.
- Taking on-demand backup and restore data from them.


# Installation

## Method 1. Download binaries

You can download binaries of a specific version from [the releases page](https://github.com/awslabs/dynein/releases). For example, below instructions are example comamnds to download the latest version in each platform.

### macOS

```
$ curl -O -L https://github.com/awslabs/dynein/releases/latest/download/dynein-macos.tar.gz
$ tar xzvf dynein-macos.tar.gz
$ mv dy /usr/local/bin/
$ dy --help
```

Currently, the above binary is automatically built on intel mac as [the GitHub Action doesn't support Apple M1 (ARM) environment yet](https://github.com/actions/virtual-environments/issues/2187).

### Linux (x86-64)

```
$ curl -O -L https://github.com/awslabs/dynein/releases/latest/download/dynein-linux.tar.gz
$ tar xzvf dynein-linux.tar.gz
$ sudo mv dy /usr/local/bin/
$ dy --help
```


## Method 2. Homebrew (macOS/Linux)

```
$ brew install dynein
```

## Method 3. Building from source

dynein is written in Rust, so you can build and install dynein with Cargo. To build dynein from source code you need to [install Rust](https://www.rust-lang.org/tools/install) as a prerequisite.

```
$ git clone [[this_git_repository_url]]
$ cd dynein
$ cargo install --locked --path .
$ ./target/release/dy --help
```

You can move the binary file named "dy" to anywhere under your `$PATH`.


# How to Use

## Prerequisites - AWS Credentials

First of all, please make sure you've already configured AWS Credentials in your environment. dynein depends on [rusoto](https://github.com/rusoto/rusoto) and rusoto [can utilize standard AWS credential toolchains](https://github.com/rusoto/rusoto/blob/master/AWS-CREDENTIALS.md) - for example `~/.aws/credentials` file, [IAM EC2 Instance Profile](https://docs.aws.amazon.com/IAM/latest/UserGuide/id_roles_use_switch-role-ec2_instance-profiles.html), or environment variables such as `AWS_DEFAULT_REGION / AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY / AWS_PROFILE`.

One convenient way to check if your AWS credential configuration is ok to use dynein is to install and try to execute [AWS CLI](https://aws.amazon.com/cli/) in your environment (e.g. `$ aws dynamodb list-tables`). Once you've [configured AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-configure.html), you should be ready to use dynein.


## Commands overview

After you installed dynein you should have a binary named `dy` in your `$PATH`. The first command you can try is `dy ls`, which lists tables you have:

```
$ dy ls --all-regions
DynamoDB tables in region: us-west-2
  EventData
  EventUsers
* Forum
  Thread
DynamoDB tables in region: us-west-1
  No table in this region.
DynamoDB tables in region: us-east-2
  UserBooks
  Users
...
```

Here `--all-regions` option enables you to iterate over all AWS regions and list all tables for you.

Next you can try `dy scan` with region and table options. `dy scan` command executes [Scan API](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Scan.html) internally to retrieve all items in the table.

```
$ dy scan --region us-west-2 --table Forum
Name             attributes
Amazon S3        {"Category":"Amazon Web Services"}
Amazon DynamoDB  {"Views":1000,"Threads":2,"Messages":4,"Category":...
```

Here `Name` is [a primary key](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.CoreComponents.html#HowItWorks.CoreComponents.PrimaryKey) of this `Forum` table and `attributes` column contains rest attributes of each item.

You don't want to pass `--region` and `--table` everytime? Let's mark the table as "currently using" with the command `dy use`.

```
$ dy use Forum --region us-west-2
```

Now you can interact with the table without specifying a target.

```
$ dy scan
Name             attributes
Amazon S3        {"Category":"Amazon Web Services"}
Amazon DynamoDB  {"Threads":2,"Views":1000,"Messages":4,"Category":...
```

To find more features, `dy help` will show you complete list of available commands.

```
$ dy --help
dynein x.x.x
dynein is a command line tool to interact with DynamoDB tables/data using concise interface.
dynein looks for config files under $HOME/.dynein/ directory.

USAGE:
    dy [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -r, --region <region>    The region to use (e.g. --region us-east-1). When using DynamodB Local, use `--region
                             local`. You can use --region option in both top-level and subcommand-level
    -t, --table <table>      Target table of the operation. You can use --table option in both top-level and subcommand-
                             level. You can store table schema locally by executing `$ dy use`, after that
                             you need not to specify --table on every command

SUBCOMMANDS:
    admin        <sub> Admin operations such as creating/updating table or GSI
    backup       Take backup of a DynamoDB table using on-demand backup
    bootstrap    Create sample tables and load test data for bootstrapping
    bwrite       Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]
    config       <sub> Manage configuration files (config.yml and cache.yml) from command line
    del          Delete an existing item. [API: DeleteItem]
    desc         Show detailed information of a table. [API: DescribeTable]
    export       Export items from a DynamoDB table and save them as CSV/JSON file
    get          Retrieve an item by specifying primary key(s). [API: GetItem]
    help         Prints this message or the help of the given subcommand(s)
    import       Import items into a DynamoDB table from CSV/JSON file
    list         List tables in the region. [API: ListTables]
    put          Create a new item, or replace an existing item. [API: PutItem]
    query        Retrieve items that match conditions. Partition key is required. [API: Query]
    restore      Restore a DynamoDB table from backup data
    scan         Retrieve items in a table without any condition. [API: Scan]
    upd          Update an existing item. [API: UpdateItem]
    use          Switch target table context. After you use the command you don't need to specify table every time,
                 but you may overwrite the target table with --table (-t) option
```

dynein consists of multiple layers of subcommands. For example, `dy admin` and `dy config` require you to give additional action to run.

```
$ dy admin --help
dy-admin x.x.x
<sub> Admin operations such as creating/updating table or GSI

USAGE:
    dy admin [OPTIONS] <SUBCOMMAND>

FLAGS: ...

OPTIONS: ...

SUBCOMMANDS:
    create    Create new DynamoDB table or GSI. [API: CreateTable, UpdateTable]
    delete    Delete a DynamoDB table or GSI. [API: DeleteTable]
    desc      Show detailed information of a table. [API: DescribeTable]
    help      Prints this message or the help of the given subcommand(s)
    list      List tables in the region. [API: ListTables]
    update    Update a DynamoDB table. [API: UpdateTable etc]
```

By executing following command, you can create a DynamoDB table.

```
$ dy admin create table mytable --keys pk,S
```


## Bootstrapping sample DynamoDB tables

The easiest way to get familiar with dynein and DynamoDB would be executing `dy bootstrap`. The `bootstrap` subcommand creates sample tables and automatically load sample data defined [here](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/AppendixSampleTables.html). After that, you'll see some sample commands to demonstrate basic usage of dynein.

```
$ dy bootstrap

Bootstrapping - dynein will creates 4 sample tables defined here:
https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/AppendixSampleTables.html

'ProductCatalog' - simple primary key table
    Id (N)

'Forum' - simple primary key table
    Name (S)

'Thread' - composite primary key table
    ForumName (S)
    Subject (S)

'Reply' - composite primary key table, with GSI named 'PostedBy-Message-Index'
    Id (S)
    ReplyDateTime (S)

...(snip logs)...

Now all tables have sample data. Try following commands to play with dynein. Enjoy!
  $ dy --region us-west-2 ls
  $ dy --region us-west-2 desc --table Thread
  $ dy --region us-west-2 scan --table Thread
  $ dy --region us-west-2 use --table Thread
  $ dy scan

After you 'use' a table like above, dynein assume you're using the same region & table, which info is stored at ~/.dynein/config.yml and ~/.dynein/cache.yml
Let's move on with the 'us-west-2' region you've just 'use'd...
  $ dy scan --table Forum
  $ dy scan -t ProductCatalog
  $ dy get -t ProductCatalog 101
  $ dy query -t Reply "Amazon DynamoDB#DynamoDB Thread 2"
  $ dy query -t Reply "Amazon DynamoDB#DynamoDB Thread 2"  --sort-key "begins_with 2015-10"
```

If you're interested in other available sample tables with data, check `dy bootstrap --list` and pass desired target to `--sample` option.


## Working with DynamoDB tables

Using dynein, you can create a table:

```
$ dy admin create table app_users --keys app_id,S user_id,S
---
name: app_users
region: us-east-1
status: CREATING
schema:
  pk: app_id (S)
  sk: user_id (S)
mode: OnDemand
capacity: ~
gsi: ~
lsi: ~
stream: ~
count: 0
size_bytes: 0
created_at: "2020-03-03T13:34:43+00:00"
```

After the table get ready (i.e. `status: CREATING` changed to `ACTIVE`), you can write-to and read-from the table.

```
$ dy use app_users
$ dy desc
---
name: app_users
region: us-east-1
status: ACTIVE
schema:
  pk: app_id (S)
  sk: user_id (S)
mode: OnDemand
capacity: ~
gsi: ~
lsi: ~
stream: ~
count: 0
size_bytes: 0
created_at: "2020-03-03T13:34:43+00:00"

$ dy put myapp 1234 --item '{"rank": 99}'
Successfully put an item to the table 'app_users'.

$ dy scan
app_id  user_id  attributes
myapp   1234     {"rank":99}
```

Similarly you can update tables with dynein.

```
$ dy admin update table app_users --mode provisioned --wcu 10 --rcu 25
```


### Infrastracture as Code - enpowered by CloudFormation

NOTE: currently this feature is under development

[Infrastracture as Code](https://www.martinfowler.com/bliki/InfrastructureAsCode.html) is a concept that you define code to provision "infrastructures", such as DynamoDB tables, with "declarative" way (On the other hand you can say `dy admin create table` and `dy admin update table` commands are "imperative" way).

To manage DynamoDB tables with "declarative" way, dynein provides `dy admin plan` and `dy admin apply` commands. Internally dynein executes [AWS CloudFormation](https://aws.amazon.com/cloudformation/) APIs to provision DynamoDB resources for you.

```
$ ls
mytable.cfn.yml

$ cat mytable.cfn.yml
Resources:
  MyDDB:
    Type: AWS::DynamoDB::Table
    Properties:
      AttributeDefinitions:
      - AttributeName: pk
        AttributeType: S
      KeySchema:
      - AttributeName: pk
        KeyType: HASH
      BillingMode: PAY_PER_REQUEST

(currently not available) $ dy admin plan
(currently not available) $ dy admin apply
```

CloudFormation manages DynamoDB tables through the resource type named [AWS::DynamoDB::Table](https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-resource-dynamodb-table.html) - visit the link for more information.


### `dy use` and `dy config` to switch/manage context

Basically it's pretty straight forward to specify table with which you want to interact with: `--table` or `-t` option. Let's say you want to scan data in the `customers` table.

```
$ dy scan --table customers
... display items in the "customers" table ...
```

However, dynein assume that tipically you're interested in only one table at some point. It means that passing table name for every single command execution is a kinf of waste of your time.

By using `dy use` for a table, you can call commands such as `scan`, `get`, `query`, and `put` without specifying table name.

```
$ dy use customers
$ dy scan
... display items in the "customers" table ...
```

In detail, when you execute `dy use` command, dynein saves your table usage information in `~/.dynein/config.yml` and caches table schema in `~/.dynein/cache.yml`. You can dump them with `dy config dump` command.

```
$ ls ~/.dynein/
cache.yml   config.yml

$ dy config dump
---
tables:
  ap-northeast-1/customers:
    region: ap-northeast-1
    name: customers
    pk:
      name: user_id
      kind: S
    sk: ~
    indexes: ~
---
using_region: ap-northeast-1
using_table: customers
```

To clear current table configuration, simply execute `dy config clear`.

```
$ dy config clear
$ dy config dump
---
tables: ~
---
using_region: ~
using_table: ~
```


## Working with DynamoDB items

As an example let's assume you have [official "Movie" sample data](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GettingStarted.Python.02.html). To prepare the table with data loaded, simply you can execute `dy bootstrap --sample movie`.

```
$ dy bootstrap --sample movie
... wait some time while dynein loading data ...
$ dy use Movie
```

After executing `dy use <your_table>` command, dynein recognize keyscheme and data type of the table. It means that some of the arguments you need to pass to access data (items) is automatically inferred when possible.


Before diving deep into each command, let me describe DynamoDB's "reserved words". One of the traps that beginners can easily fall into is that you cannot use [certain reserved words](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ReservedWords.html) in DynamoDB APIs. DynamoDB reserved words contains common words that you may want to use in your application. For example "name", "year", "url", "token", "error", "date", "group" -- all of them are reserved so you cannot use them in expressions directly.

Normally, to use reserved words in expressions, you need to use placeholders instead of actual values. For more information, see [Expression Attribute Names](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.ExpressionAttributeNames.html) and [Expression Attribute Values](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.ExpressionAttributeValues.html).

To make it easy to interact with DynamoDB items, dynein automatically replace reserved words to placeholders internally - thus you don't need to care about it.

### Read

#### `dy scan`

The simplest command would be `dy scan`, which list items in a table.

```
$ dy scan --limit 10
year  title                  attributes
1933  King Kong              {"info":{"actors":["Bruce Cabot","Fay Wray","Rober...
1944  Arsenic and Old Lace   {"info":{"actors":["Cary Grant","Priscilla Lane","...
1944  Double Indemnity       {"info":{"actors":["Barbara Stanwyck","Edward G. R...
1944  I'll Be Seeing You     {"info":{"actors":["Ginger Rogers","Joseph Cotten"...
1944  Lifeboat               {"info":{"actors":["John Hodiak","Tallulah Bankhea...
1958  Cat on a Hot Tin Roof  {"info":{"actors":["Burl Ives","Elizabeth Taylor",...
1958  Monster on the Campus  {"info":{"actors":["Arthur Franz","Joanna Moore","...
1958  No Time for Sergeants  {"info":{"actors":["Andy Griffith","Myron McCormic...
1958  Teacher's Pet          {"info":{"actors":["Clark Gable","Doris Day","Gig ...
1958  Touch of Evil          {"info":{"actors":["Charlton Heston","Janet Leigh"...
```


#### `dy get`

You may notice that non-key attributes are trimmed in above `dy scan` results. To get full details of a single item, you use `dy get` command with primary key. As the "Movie" table is defined with [composite primary key](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.CoreComponents.html#HowItWorks.CoreComponents.PrimaryKey), you have to pass "partition key (= year) and "sort key (= title)" to identify an item.

```
$ dy desc
---
name: Movie
region: us-west-2
status: ACTIVE
schema:
  pk: year (N)     <<<<=== "year" and
  sk: title (S)    <<<<=== "title" are the information to identify an item.
...

$ dy get 1958 "Touch of Evil"
{
  "info": {
    "actors": [
      "Charlton Heston",
      "Janet Leigh",
      "Orson Welles"
    ],
    "directors": [
      "Orson Welles"
    ],
    "genres": [
      "Crime",
      "Film-Noir",
      "Thriller"
    ],
    "image_url": "http://ia.media-imdb.com/images/M/MV5BNjMwODI0ODg1Nl5BMl5BanBnXkFtZTcwMzgzNjk3OA@@._V1_SX400_.jpg",
    "plot": "A stark, perverse story of murder, kidnapping, and police corruption in a Mexican border town.",
    "rank": 3843,
    "rating": 8.2,
    "release_date": "1958-04-23T00:00:00Z",
    "running_time_secs": 5700
  },
  "title": "Touch of Evil",
  "year": 1958
}
```

Note that if your table has a simple primary key, the only argument you need to pass is a partition key (e.g. `dy get yourpk`), as the only information DynamoDB requires to identify an item is only a partition key.


#### `dy query`

Next command you can try to retrieve items would be: `dy query`. By passing a partition key, `dy query` returns items that have the specified partition key.

```
$ dy query 1960
year  title                  attributes
1960  A bout de souffle      {"info":{"actors":["Daniel Boulanger","Jean Seberg...
1960  La dolce vita          {"info":{"actors":["Anita Ekberg","Anouk Aimee","M...
1960  Ocean's Eleven         {"info":{"actors":["Dean Martin","Frank Sinatra","...
1960  Plein soleil           {"info":{"actors":["Alain Delon","Marie Laforet","...
1960  Spartacus              {"info":{"actors":["Jean Simmons","Kirk Douglas","...
1960  The Apartment          {"info":{"actors":["Fred MacMurray","Jack Lemmon",...
1960  The Magnificent Seven  {"info":{"actors":["Charles Bronson","Steve McQuee...
1960  The Time Machine       {"info":{"actors":["Alan Young","Rod Taylor","Yvet...
```

Also you can add more conditions on sort key. For example, following command would return items that has sort keys begins with "The".

```
$ dy query 1960 --sort-key "begins_with The"
year  title                  attributes
1960  The Apartment          {"info":{"actors":["Fred MacMurray","Jack Lemmon",...
1960  The Magnificent Seven  {"info":{"actors":["Charles Bronson","Steve McQuee...
1960  The Time Machine       {"info":{"actors":["Alan Young","Rod Taylor","Yvet...
```

Other examples for the `--sort-key` option of `dy query` are: `--sort-key "= 42"`, `--sort-key "> 42"`, or `--sort-key "between 10 and 42"`. For more details please visit [a public document "Working with Queries"](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Query.html) and [DynamoDB Query API reference](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Query.html).


### Write

dynein provides subcommands to write to DynamoDB tables as well.


#### `dy put`

`dy put` internally calls [PutItem API](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_PutItem.html) and save an item to a target table.
To save an item, you need to pass at least primary key that identifies an item among the table.

```
$ dy admin create table write_test --keys id,N
$ dy use write_test

$ dy put 123
Successfully put an item to the table 'write_test'.
$ dy scan
id  attributes
123
```

Additionally, you can include an item body (non-key attributes) by passing `--item` or `-i` option.
The `--item` option takes a JSON-style expression with extended syntax.

```
$ dy put 456 --item '{"a": 9, "b": "str"}'
Successfully put an item to the table 'write_test'.

$ dy scan
id  attributes
123
456  {"a":9,"b":"str"}
```

As the parameter of the `--item` option automatically transforms into DynamoDB-style JSON syntax,
writing items into a table would be more straightforward than AWS CLI.
See the following comparison:

```
$ dy put 789 --item '{"a": 9, "b": "str"}'

// The above dynein command is equivalent to AWS CLI's following command:
$ aws dynamodb put-item --table-name write_test --item '{"id": {"N": "456"}, "a": {"N": "9"}, "b": {"S": "str"}}'
```

Please see the [dynein format](./docs/format.md) for details of JSON-style data.
To summarize, in addition to the string ("S") and number ("N"), dynein also supports other data types such as boolean ("BOOL"),
null ("NULL"), binary ("B"), string set ("SS"), number set ("NS"), binary set("BS"),
list ("L"), and nested object ("M").

```
$ dy put 999 --item '{"myfield": "is", "nested": {"can": true, "go": false, "deep": [1,2,{"this_is_set": <<"x","y","z">>}]}}'
Successfully put an item to the table 'write_test'.
$ dy get 999
{
  "nested": {
    "can": true,
    "deep": [
      1,
      2,
      {
        "this_is_set": [
          "x",
          "y",
          "z"
        ]
      }
    ],
    "go": false
  },
  "myfield": "is",
  "id": 999
}
```


#### `dy upd`

`dy upd` command internally executes [UpdateItem API](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html) and you use "[update expression](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html)" to update an item. Recommended way to update items is use `SET` and `REMOVE` in update expression.

with dynein, you can use `--set` or `--remove` option. Here's an exmaple:

```bash
$ dy put 42 -i '{"flag": true}'
Successfully put an item to the table 'test'.

$ dy get 42
{
  "flag": true,
  "id": 42
}

# Set a boolean
$ dy upd 42 --set "flag = false"
Successfully updated an item in the table 'write_test'.

$ dy get 42
{
  "flag": false,
  "id": 42
}

# Set a string value
$ dy upd 42 --set "date = '2022-02-22T22:22:22Z'"
$ dy get 42
{
  "date": "2022-02-22T22:22:22Z",
  "id": "42",
  "flag": false
}

# Set a number
$ dy upd 42 --set 'pi = +3.14159265358979323846'
$ dy get 42
{
  "date": "2022-02-22T22:22:22Z",
  "pi": 3.141592653589793,
  "id": "42",
  "flag": false
}

# You can apply an addition (+) and a subtraction (-) to the numbers. Please note that DynamoDB does not support unary operator (+, -), multiplication and division.
$ dy upd 42 --set 'pi = pi + 10'
$ dy get 42 | jq .pi
13.141592653589793

$ dy upd 42 --set 'pi = 1 - pi'
$ dy get 42 | jq .pi
-12.141592653589793
```

Next let me show an example to use `--remove`. Note that `--remove` in `dy upd` command never remove "item" itself, instead `--remove` just removes an "attribute".

```bash
$ dy upd 42 --remove flag
Successfully updated an item in the table 'write_test'.

$ dy get 42
{
  "id": "42",
  "date": "2022-02-22T22:22:22Z",
  "pi": 3.141592653589793
}

# You can remove multiple attributes
$ dy upd 42 --remove "date, pi"
$ dy get 42
{
  "id": "42"
}
```

DynamoDB supports a list type which has order. Let's try it with dynein.

```bash
# Create an empty list
$ dy upd 42 --set "list = []"
$ dy get 42
{
  "id": "42",
  "list": []
}

# Add an elements into the list
$ dy upd 42 --set "list = list_append(list, ['item1'])"
$ dy get 42
{
  "id": "42",
  "list": [
    "item1"
  ]
}

# Prepend an element to the list
$ dy upd 42 --set "list = list_append(['item0'], list)"
$ dy get 42 | jq .list
[
  "item0",
  "item1"
]

# Add more elements
$ dy upd 42 --set "list = list_append(list, ['item2', 'item3'])"
$ dy get 42 | jq .list
[
  "item0",
  "item1",
  "item2",
  "item3"
]

# You can directly modify the list element
$ dy upd 42 --set "list[0] = 'item0 modified'"
$ dy get 42 | jq .list
[
  "item0 modified",
  "item1",
  "item2",
  "item3"
]

# Delete the element from the list
$ dy upd 42 --remove 'list[0]'
$ dy get 42 | jq .list
[
  "item1",
  "item2",
  "item3"
]

# Remove the list attribute
$ dy upd 42 --remove list
$ dy get 42
{
  "id": "42"
}
```

Furthermore, it's possible to update multiple attributes simultaneously.

```bash
# Set numbers
$ dy upd 42 --set "n1 = 0, n2 = 1"
$ dy get 42
{
  "n2": 1,
  "id": "42",
  "n1": 0
}

# Calculate Fibonacci numbers
$ dy upd 42 --set "n1 = n2, n2 = n1 + n2"
$ dy get 42 | jq -c '[.n1,.n2]'
[1,1]

# Calculate the next value
$ dy upd 42 --set "n1 = n2, n2 = n1 + n2"
$ dy get 42 | jq -c '[.n1,.n2]'
[1,2]

# You can get more sequence
$ dy upd 42 --set "n1 = n2, n2 = n1 + n2"
$ dy get 42 | jq -c '[.n1,.n2]'
[2,3]

$ dy upd 42 --set "n1 = n2, n2 = n1 + n2"
$ dy get 42 | jq -c '[.n1,.n2]'
[3,5]

# Clean up the attributes
$ dy upd 42 --remove "n1,n2"
$ dy get 42
{
  "id": "42"
}
```

As demonstrated in `dy put`, map type expresses nested values. Let's manipulate it with dynein.

```bash
$ dy upd 42 --set 'ProductReviews = {"metadata": {"counts": 0, "average": null}}'
$ dy get 42
{
  "id": "42",
  "ProductReviews": {
    "metadata": {
      "average": null,
      "counts": 0
    }
  }
}

$ dy upd 42 --set 'ProductReviews.FiveStar = ["Excellent product"], ProductReviews.metadata = {"average": 5, "sum": 5, "counts": 1}'
$ dy get 42
{
  "id": "42",
  "ProductReviews": {
    "FiveStar": [
      "Excellent product"
    ],
    "metadata": {
      "average": 5,
      "counts": 1,
      "sum": 5
    }
  }
}

$ dy upd 42 --set 'ProductReviews.FiveStar[1] = "Very happy with my purchase", ProductReviews.ThreeStar = ["Just OK - not that great"], ProductReviews.metadata = {"average": 4.3, "sum": 13, "counts": 3}'
$ dy get 42
{
  "id": "42",
  "ProductReviews": {
    "FiveStar": [
      "Excellent product",
      "Very happy with my purchase"
    ],
    "ThreeStar": [
      "Just OK - not that great"
    ],
    "metadata": {
      "average": 4.3,
      "counts": 3,
      "sum": 13
    }
  }
}

$ dy upd 42 --set 'ProductReviews.OneStar = if_not_exists(ProductReviews.OneStar, [])'
$ dy get 42
{
  "id": "42",
  "ProductReviews": {
    "FiveStar": [
      "Excellent product",
      "Very happy with my purchase"
    ],
    "OneStar": [],
    "ThreeStar": [
      "Just OK - not that great"
    ],
    "metadata": {
      "average": 4.3,
      "counts": 3,
      "sum": 13
    }
  }
}

$ dy upd 42 --set 'ProductReviews.OneStar = list_append(ProductReviews.OneStar, ["Broken"]), ProductReviews.metadata = {"average": 3.5, "sum": 14, "counts": 4}'
$ dy get 42
{
  "ProductReviews": {
    "FiveStar": [
      "Excellent product",
      "Very happy with my purchase"
    ],
    "OneStar": [
      "Broken"
    ],
    "ThreeStar": [
      "Just OK - not that great"
    ],
    "metadata": {
      "average": 3.5,
      "counts": 4,
      "sum": 14
    }
  },
  "id": "42"
}

$ dy upd 42 --remove ProductReviews
$ dy get 42
{
  "id": "42"
}
```

dynein has a special command named `--atomic-counter`. It increases specified number attribute by `1`.

```bash
$ dy get 52
{
  "age": 28,
  "name": "John",
  "id": 52
}

$ dy upd 52 --atomic-counter age
Successfully updated an item in the table 'write_test'.

$ dy get 52
{
  "age": 29,
  "id": 52,
  "name": "John"
}
```

##### Supported String Literals

There are two types of string literals that you can use:

- Double quote (`"`): Double quoted string literals support escape sequences such as `\0`, `\r`, `\n`, `\t`, `\\`, `\"`, and `\'`. Each of them represents a null character, carriage return, new line, horizontal tab, backslash, double quote, and single quote, respectively. If you need to include a double quote inside the literal, you must escape it.
- Single quote (`'`): Single-quoted string literals are interpreted as you input them. However, you cannot specify a string that includes a single quote. In such cases, you can use a double-quoted string literal.

##### Supported Functions

The `upd` command supports the following functions:

- `list_append`: This function is used to concatenate two lists, where each list can be a literal or a path to an attribute. When you call `list_append([l1,l2], [l3,l4,l5])`, it will return `[l1,l2,l3,l4,l5]`.
- `if_not_exists`: This fungiction allows you to set a default value for the `null` case. The left-hand argument represents the path to an attribute, while the right-hand argument specifies the default value for the `null`.

For more details, please refer to the [official documentation](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html#Expressions.UpdateExpressions.SET.UpdatingListElements).

#### Quoting a Path of an Attribute

Sometimes, you may need to specify a path that includes a space or special characters that are not allowed by dynein. In such cases, you can use backticks to quote the path. For example, consider the following item:

```json
{
  "id": {"S":  "55"},
  "map": {
    "M": {
      "Do you have spaces?": {
        "S": "Yes"
      },
      "Dou you `?": {
        "S": "Yes"
      },
      "路径": {"S": "Chinese"},
      "パス": {"S": "Japanese"},
      "경로": {"S": "Korean"}
    }
  }
}
```

You can specify a path using the following syntax:

* ```dy upd 55 --set 'map.`Do you have spaces?` = "Allowed"'```
* ```dy upd 55 --set 'map.`Dou you ``?` = "Maybe"'```

As demonstrated above, you can use double backticks (``) to represent a backtick (`) within the path.

Please note that you may not need to escape non-ASCII paths like CJK characters. For example, you can specify `路径`, `パス`, and `경로` without quotes. Dynein allows you to specify a path where the first character belongs to the `ID_Start` class and the subsequent characters belong to the `ID_Continue` class without requiring escape sequences. These classes are defined by the [Unicode standard](https://www.unicode.org/reports/tr31/tr31-37.html). The following examples illustrate this:

* ```dy upd 55 --set 'map.路径 = "A word of Chinese"'```
* ```dy upd 55 --set 'map.パス = "A word of Japanese"'```
* ```dy upd 55 --set 'map.경로 = "A word of Korean"'```

#### `dy del`

To delete an item, you use `dy del` command with primary key to identify an item.

```bash
$ dy get 42
{ "id": 42 }
$ dy del 42
Successfully deleted an item from the table 'write_test'.
$ dy get 42
No item found.
```


## Working with Indexes

DynamoDB provides flexible way to query data efficiently by utilizing [Secondary Index features](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/SecondaryIndexes.html). There're two types of secondary indexes: GSI (Global Secondary Index) and LSI (Local Secondary Index), but you can create LSI only when creating a table.

With dynein, you can add GSI to existing table.

```
$ dy admin create index top_rank_users_index --keys rank,N --table app_users
---
name: app_users
region: us-west-2
status: UPDATING
schema:
  pk: app_id (S)
  sk: user_id (S)
mode: OnDemand
capacity: ~
gsi:
  - name: top_rank_users_index
    schema:
      pk: rank (N)
      sk: ~
    capacity: ~
lsi: ~
stream: ~
count: 0
size_bytes: 0
created_at: "2020-06-02T14:22:56+00:00"

$ dy use app_users
$ dy scan --index top_rank_users_index
```

## Import/Export for DynamoDB items

### `dy export`

You can export DynamoDB items into JSON or CSV file. As the default format is json, you can simply call following command to export:

```
$ dy export --table Reply --format json --output-file out.json
$ cat out.json
[
  {
    "PostedBy": "User A",
    "ReplyDateTime": "2015-09-15T19:58:22.947Z",
    "Id": "Amazon DynamoDB#DynamoDB Thread 1",
    "Message": "DynamoDB Thread 1 Reply 1 text"
  },
  {
    "Id": "Amazon DynamoDB#DynamoDB Thread 1",
...
```

No `--format` option means `--format json`. If you want to dump data in oneline, try `--format json-compact`. Or, if you want to export in [JSONL (JSON Lines)](http://jsonlines.org/), i.e. "one JSON item per one line" style, `--format jsonl` is also available.

```
$ dy export --table Reply --format jsonl --output-file out.jsonl
$ cat out.jsonl
{"PostedBy":"User A","ReplyDateTime":"2015-09-15T19:58:22.947Z","Message":"DynamoDB Thread 1 Reply 1 text","Id":"Amazon DynamoDB#DynamoDB Thread 1"}
{"PostedBy":"User B","Message":"DynamoDB Thread 1 Reply 2 text","ReplyDateTime":"2015-09-22T19:58:22.947Z","Id":"Amazon DynamoDB#DynamoDB Thread 1"}
...
```

When export data to CSV, primary key(s) are exported by default. You can explicitly pass additional attributes to export.

```
$ dy export --table Reply --output-file out.csv --format csv --attributes PostedBy,Message
$ cat out.csv
Id,ReplyDateTime,PostedBy,Message
"Amazon DynamoDB#DynamoDB Thread 1","2015-09-15T19:58:22.947Z","User A","DynamoDB Thread 1 Reply 1 text"
...
```

### `dy import`

To import data into a table, you use with specified `--format` option. Here default format is JSON like `dy export`.

```
$ dy import --table target_movie --format json --input-file movie.json
```

#### Enable set type inference

Dynein provides the type inference for set types (number set, string set) for backward compatibility.
If you want to retain the inference behavior before 0.3.0, you can use `--enable-set-inference` option.

Without option, all JSON lists are inferred as list type.

```bash
$ cat load.json
{"pk":1,"string-set":["1","2","3"]}
{"pk":2,"number-set":[1,2,3]}
{"pk":3,"list":["1",2,"3"]}

$ dy admin create table target_movie -k pk,N
$ dy import --table target_movie --format jsonl --input-file load.json
$ aws dynamodb get-item --table-name target_movie --key '{"pk":{"N":"1"}}'
{
    "Item": {
        "string-set": {
            "L": [
                {
                    "S": "1"
                },
                {
                    "S": "2"
                },
                {
                    "S": "3"
                }
            ]
        },
        "pk": {
            "N": "1"
        }
    }
}
```

With `--enable-set-inference` option, JSON lists are inferred based on their content.

```bash
$ dy admin create table target_movie2 -k pk,N
$ dy import --table target_movie --format jsonl --enable-set-inference --input-file load.json
$ aws dynamodb get-item eu-north-1 --table-name target_movie --key '{"pk":{"N":"1"}}'
{
    "Item": {
        "string-set": {
            "SS": [
                "1",
                "2",
                "3"
            ]
        },
        "pk": {
            "N": "1"
        }
    }
}

$ aws dynamodb get-item --table-name target_movie --key '{"pk":{"N":"2"}}'
{
    "Item": {
        "pk": {
            "N": "2"
        },
        "number-set": {
            "NS": [
                "3",
                "2",
                "1"
            ]
        }
    }
}

$ aws dynamodb get-item --table-name target_movie --key '{"pk":{"N":"3"}}'
{
    "Item": {
        "pk": {
            "N": "3"
        },
        "list": {
            "L": [
                {
                    "S": "1"
                },
                {
                    "N": "2"
                },
                {
                    "S": "3"
                }
            ]
        }
    }
}
```

## Using DynamoDB Local with `--region local` option

DynamoDB provides [free tier](https://aws.amazon.com/free/?all-free-tier.sort-by=item.additionalFields.SortRank&all-free-tier.sort-order=asc&awsf.Free%20Tier%20Categories=*all&all-free-tier.q=dynamodb&all-free-tier.q_operator=AND) that consists of [25 GB of storage and 25 WCU/RCU](https://aws.amazon.com/dynamodb/pricing/provisioned/) which is enough to handle up to 200M requests per month. However, if you're already using DynamoDB in your account and worrying about additional costs by getting started with dynein, you can use [DynamoDB Local](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBLocal.html).

Yes, dynein supports DynamoDB Local. The only difference you need to add would be `--region local` option for every command. To get start with DynamoDB Local [Docker version](https://hub.docker.com/r/amazon/dynamodb-local) with dynein is quite simple as follows.

Simply you can run docker container and expose 8000 port by following command.

```
$ docker run -p 8000:8000 -d amazon/dynamodb-local
```

Optionally, if you prefer Kubernetes, you can use manifest file in this repository.

```
$ kubectl apply -f k8s-deploy-dynamodb-local.yml
$ kubectl port-forward deployment/dynamodb 8000:8000
```

Now you can interact with DynamoDB Local with `--region local` option.

```
$ dy --region local admin create table localdb --keys pk
$ dy --region local use -t localdb
$ dy put firstItem
$ dy put secondItem
$ dy scan
```


# Misc

## Development
For development, we use `rustfmt` and `clippy` to maintain the quality of our source code.
Before starting, please ensure both components are installed;

```shell
rustup component add rustfmt clippy
```

Additionally, we use `pre-commit` hooks to execute automated linting and basic checks.
Please set it up before creating a commit;

```shell
brew install pre-commit # (or appropriate for your platform: https://pre-commit.com/)
pre-commit install
```

We use [trycmd](https://crates.io/crates/trycmd) to conduct snapshot testing for CLI.
If the snapshot is needed to be updated, run command;

MacOS and Linux
```shell
TRYCMD=overwrite cargo test --test cli_tests
```

Windows (PowerShell)
```powershell
$Env:TRYCMD='overwrite'
cargo test --test cli_tests
[Environment]::SetEnvironmentVariable('TRYCMD',$null)
```

Please note that we use different snapshots for the Windows environment.

### Bot
If you want to update snapshots of commands, you can use bot command `/snapshot` in your pull request.
Please note that you must type exactly as written.

## Asides

dynein is named after [a motor protein](https://en.wikipedia.org/wiki/Dynein).

## Troubleshooting

If you encounter troubles, the first option worth trying is removing files in `~/.dynein/` or the directory itself. Doing this just clears "cached" info stored locally for dynein and won't affect your data stored in DynamoDB tables.

```
$ rm -rf ~/.dynein/
```

To see verbose output for troubleshooting purpose, you can change log level by `RUST_LOG` environment variable. For example:

```
$ RUST_LOG=debug RUST_BACKTRACE=1 dy scan --table your_table
```

## Ideas for future works

- `dy admin plan` & `dy admin apply` commands to manage tables through CloudFormation.
  - These subcommand names are inspired by [HashiCorp's Terraform](https://www.terraform.io/).
- Linux's `top` -like experience to monitor table status. e.g. `dy top tables`
  - inspired by `kubectl top nodes`
  - implementation:  (CloudWatch metrics such as Consumed WCU/RCU, SuccessfulRequestLatency, ReplicationLatency for GT etc)
- Shell (bash/zsh) completion
- Retrieving control plane APIs, integrated with CloudTrail
- `dy logs` command to retrieving data plane API logs via DynamoDB Streams (write APIs only)
  -  `tail -f` -ish usability. e.g. `dy logs -f mytable`
- `truncate` command to delete all data in a table
- Support Transaction APIs (TransactGetItems, TransactWriteItems)
- simple load testing. e.g. `dy load --tps 100`
- import/export tool supports LTSV, TSV
- PITR configuration enable/disable (UpdateContinuousBackups) and exporting/restoring tables ([ExportTableToPointInTime](https://aws.amazon.com/blogs/aws/new-export-amazon-dynamodb-table-data-to-data-lake-amazon-s3/), RestoreTableToPointInTime)
