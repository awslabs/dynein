# `dy query` command
In dynein, you can use `dy query` command to retrieve items that match the specified condition.

You can specify the following conditions:

* Partition key
* Sort key condition

The partition key is a required argument and only supports exact equal matches.
On the other hand, the sort key condition is the optional argument.
When you do not specify the sort key condition, DynamoDB returns all items whose partition key matches the specified value.

For example, you can use the following command to retrieve the items whose partition key is `0001`.

```bash
dy admin create table query-format --keys pk,S sk,S
dy use query-format
dy put 0001 01
dy put 0001 02
dy put 0001 11
dy put 0001 12
dy query 0001
```

Please note that partition and sort keys are defined as string type (S).
The output of `dy query 0001` must be the following:

```log
pk    sk  attributes
0001  01
0001  02
0001  11
0001  12
```

If you need the items whose sort key starts with `0`, you can use the following command.

```bash
dy query 0001 -s 'begins_with "0"'
```

The output is as follows:

```log
pk    sk  attributes
0001  01
0001  02
```

As mentioned earlier, you must provide the partition key for the query command.
If you find another primary key is beneficial for your access patterns, you can use [secondary indexes](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/SecondaryIndexes.html).

For more details regarding the query operation, please visit AWS documents ["Working with Queries"](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Query.html) and [DynamoDB Query API reference](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Query.html).

### Supported comparison operators
In sort key condition, you can use the following operations:


| Comparison operators               | Condition meanings                                                       |
|------------------------------------|--------------------------------------------------------------------------|
| `= A` or `== A`                    | a value of the sort key is equal to `A`                                  |
| `< A`                              | a value of the sort key is less than `A`                                 |
| `<= A`                             | a value of the sort key is less than or equal to `A`                     |
| `> A`                              | a value of the sort key is greater than `A`                              |
| `>= A`                             | a value of the sort key is greater than or equal to`A`                   |
| `BETWEEN A AND B` or `BETWEEN A B` | a value of the sort key is between `A` and `B` (inclusive at both sides) |
| `BEGINS_WITH A`                    | a value of the sort key has a prefix A                                   |

The keywords `BETWEEN`, `AND`, and `BEGINS_WITH` are case-insensitive.

## Sort key format
Dynein provides two types of sort key formats: strict and non-strict.
By default, dynein tries to parse both input formats.
The first try is the strict format, and the second is the non-strict format.
Therefore, if your input matches the strict format, it is not parsed as the non-strict format.

You can use the `--strict` option to enforce the use of the strict format.
Dynein raises an error if you provide a non-strict input.
On the other hand, when the `--no-strict` option is provided, dynein checks the same as the default, even if you specify the strict mode in your config.

### Strict format
In strict format, you must specify a correct value in [dynein format](./format.md) for the right-hand value of comparison operators.
Providing the correct type is your responsibility.
For example, you can use the following command to retrieve items from the table whose sort key is string type.

```bash
dy query --strict 0001 -s '< "1"'
```

If you do not specify the correct type, dynein raises an error.
For example, you undergo an error in the following command with the same table.

```bash
dy query --strict 0001 -s '<= 1'
```

Raw `1` means a value of the number type, which does not align with the table definition, which expects the string type.
Therefore, dynein does not accept this input.

### Non-strict format
In the non-strict format, your right-hand value does not need to align with the table definition.
Dynein tries to reach your intention as much as possible.
This means that parsing for the non-strict format may change to improve user experience in the future, and **it may not be treated as breaking changes**.

For example, the following command succeeds against the table whose sort key is string type.

```bash
dy query --non-strict 0001 -s '<= 1'
```

The dynein understands the above expression the same as follows:

```bash
dy query --strict 0001 -s '<= "1"'
```

Also, when you want to specify the equal operator, you can specify its value directly.

```bash
dy query --non-strict 0001 -s '01'
```

It is semantically equivalent to the following command:

```bash
dy query --strict 0001 -s '= "01"'
```

### Configuration
You can change the default behavior whether dynein accepts a non-strict format.
If you want to enforce the strict format, you can utilize the `strict_mode` option in `~/.dynein/config.yml`.

```yaml
query:
  strict_mode: true
```
