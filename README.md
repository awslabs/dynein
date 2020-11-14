dynein - DynamoDB CLI
========================================

**dynein** /daɪ.nɪn/ is a command line interface for [Amazon DynamoDB](https://aws.amazon.com/dynamodb/) written in Rust. dynein is designed to make it simple to interact with DynamoDB tables/items from terminal.

<!-- TOC -->

- [Why use dynein?](#why-use-dynein)
    - [Less Typing](#less-typing)
    - [Quick Start](#quick-start)
    - [For day-to-day tasks](#for-day-to-day-tasks)
- [Installation](#installation)
    - [Method 1. HomeBrew (MacOS)](#method-1-homebrew-macos)
    - [Method 2. Download a binary](#method-2-download-a-binary)
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
    - [Ideas for future works](#ideas-for-future-works)

<!-- /TOC -->


# Why use dynein?

## Less Typing

- Auto completion for table/keyDefinitions enables using DynamoDB with minimum arguments. e.g. to get an item: `dy get abc`
- Switching table context by RDBMS-ish "use".
- Prefer standard JSON ( `{"id": 123}` ) over DynamoDB JSON ( `{"id": {"N": "123"}}` ).

## Quick Start

- Bootstrap command enables you to launch sample table with sample data sets.
- Supports DynamoDB Local and you can test DyanmoDB at no charge.

## For day-to-day tasks

- Import/Export by single command: export DynamoDB items to CSV/JSON files and conversely, import them into tables.
- Taking on-demand backup and restore data from them.


# Installation

## Method 1. HomeBrew (MacOS)

(currently not available)

## Method 2. Download a binary

(currently not available)

## Method 3. Building from source

dynein is written in Rust, so you can build and install dynein with Cargo. To build dynein from source code you need to [install Rust](https://www.rust-lang.org/tools/install) as a prerequisite.

```
$ git clone [[this_git_repository_url]]
$ cd dynein
$ cargo install --path .
$ ./target/release/dy help
```

You can move the binary file named "dy" to anywhere under your `$PATH`.


# How to Use

## Prerequisites - AWS Credentials

First of all, please make sure you've already configured AWS Credentials in your environment. dynein depends on [rusoto](https://github.com/rusoto/rusoto) and rusoto [can utilize standard AWS credential toolchains](https://github.com/rusoto/rusoto/blob/master/AWS-CREDENTIALS.md) - for example `~/.aws/credentials` file, [IAM EC2 Instance Profile](https://docs.aws.amazon.com/IAM/latest/UserGuide/id_roles_use_switch-role-ec2_instance-profiles.html), or environment variables such as `AWS_DEFAULT_REGION / AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY`.

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
$ dy use --region us-west-2 --table Forum
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
dynein looks for a config file under $HOME/.dynein directory. For more info: https://github.com/thash/dynein

USAGE:
    dy [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
    -r, --region <region>
            The region to use. When using DynamodB Local, `--region local` You can use --region option in both top-level
            and subcommand-level
    -t, --table <table>
            Target table. By executing `$ dy use -t <table>` you can omit --table on every command. You can use --table
            option in both top-level and subcommand-level

SUBCOMMANDS:
    admin        <sub> Admin operations such as creating/updating table or index
    backup       Take backup of a DynamoDB table using on-demand backup
    bootstrap    Create sample tables and load test data for bootstrapping
    bwrite       Put or Delete multiple items at one time, up to 25 requests. [API: BatchWriteItem]
    config       <sub> Manage configuration file (~/.dynein/config.yml) from command line
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
    use          Switch target table context. You can overwrite the context with --table
```

dynein consists of multiple layers of subcommands. For example, `dy admin` and `dy config` require you to give additional action to run.

```
$ dy admin --help
dy-admin 0.1.0
<sub> Admin operations such as creating/updating table or GSI

USAGE:
    dy admin [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -r, --region <region>    The region to use. When using DynamodB Local, `--region local` You can use --region option
                             in both top-level and subcommand-level
    -t, --table <table>      Target table. By executing `$ dy use -t <table>` you can omit --table on every command. You
                             can use --table option in both top-level and subcommand-level

SUBCOMMANDS:
    create    Create new DynamoDB table or GSI
    delete    Delete a DynamoDB table or GSI
    desc      Show detailed information of a table. [API: DescribeTable]
    help      Prints this message or the help of the given subcommand(s)
    list      List tables in the region. [API: ListTables]
```

By executing following command, you can create a DynamoDB.

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

After you 'use' a table like above, dynein assume you're using the same region & table, which info is stored at ~/.dynein/config.yml
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

After the table get ready (= `ACTIVE` status), you can write-to and read-from the table.

```
$ dy use -t app_users
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
(currently not available) $ dy admin update table --mode provisioned --wcu 10 --rcu 25
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
$ dy use -t customers
$ dy scan
... display items in the "customers" table ...
```

In detail, when you execute `dy use` command, dynein saves your table usage information under `~/.dynein/config.yml`. You can dump the info with `dy config dump` command.

```
$ dy config dump
---
table:
  region: ap-northeast-1
  name: customers
  pk:
    name: user_id
    kind: S
  sk: ~
  indexes: ~
```

To clear current table configuration, simply execute `dy config clear`.

```
$ dy config clear
$ dy config dump
---
table: ~
```


## Working with DynamoDB items

As an example let's assume you have [official "Movie" sample data](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GettingStarted.Python.02.html). To prepare the table with data loaded, simply you can execute `dy bootstrap --sample movie`.

```
$ dy bootstrap --sample movie
... wait some time while dynein loading data ...
$ dy use -t Movie
```

After executing `dy use -t <your_table>` command, dynein recognize keyscheme and data type of the table. It means that some of the arguments you need to pass to access data (items) is automatically inferred when possible.


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

`dy put` internally calls [PutItem API](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_PutItem.html) and save an item to a target table. To save an item, you need to pass at least primary key that identifies an item among the table.

```
$ dy admin create table write_test --keys id,N
$ dy use -t write_test

$ dy put 123
Successfully put an item to the table 'write_test'.
$ dy scan
id  attributes
123
```

Additionally you can include item body (non-key attributes) by passing `--item` or `-i` option. The `--item` option takes JSON style syntax.

```
$ dy put 456 --item '{"a": 9, "b": "str"}'
Successfully put an item to the table 'write_test'.

$ dy scan
id  attributes
123
456  {"a":9,"b":"str"}
```

As dynein's `--item` option automatically transform standard JSON into DynamoDB style JSON syntax, writing items into a table would be simpler than AWS CLI. See following comparison:

```
$ dy put 789 --item '{"a": 9, "b": "str"}'

// The above dynein command is equivalent to AWS CLI's following command:
$ aws dynamodb put-item --table-name write_test --item '{"id": {"N": "456"}, "a": {"N": "9"}, "b": {"S": "str"}}'
```

Finally, in addition to the string ("S") and nubmer ("N"), dynein also supports other data types such as boolean ("BOOL"), null ("NULL"), string set ("SS"), number set ("NS"), list ("L"),  and nested object ("M").

```
$ dy put 999 --item '{"myfield": "is", "nested": {"can": true, "go": false, "deep": [1,2,{"this_is_set": ["x","y","z"]}]}}'
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

```
$ dy get 42
{
  "flag": true,
  "id": 42
}

$ dy upd 42 --set "flag = false"
Successfully updated an item in the table 'write_test'.

$ dy get 42
{
  "flag": false,
  "id": 42
}
```

Next let me show an example to use `--remove`. Note that `--remove` in `dy upd` command never remove "item" itself, instead `--remove` just removes an "attribute".

```
$ dy upd 42 --remove "flag"
Successfully updated an item in the table 'write_test'.

$ dy get 42
{
  "id": 42
}
```

dynein has a special command named `--atomic-counter`. It increases specified number atribute by `1`.

```
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

#### `dy del`

To delete an item, you use `dy del` command with primary key to identify an item.

```
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

$ dy use -t app_users
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
$ dy --region local table create localdb --keys pk
$ dy --region local use localdb
$ dy put firstItem
$ dy put secondItem
$ dy scan
```


# Misc

dynein is named after [a motor protein](https://en.wikipedia.org/wiki/Dynein).

## Ideas for future works

Sorted by feasibility/simplicity.

- `dy admin update table` command
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
