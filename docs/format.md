# Dynein format

Dynein uses a JSON-like format called dynein format to express an item.
Dynein format is not intended to be compatible with JSON; however, valid JSON should be parsed correctly as dynein format.
Dynein format is designed to be easy to write and understand its data type at a glance.
This format is inspired by [PartiQL](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ql-reference.data-types.html), but we focus on more usability than compatibility for SQL.

NOTE: The current implementation cannot read all valid JSON. This issue should be fixed in the future.

## Supported types

Dynein format supports all DynamoDB types. In other words, you can use the following types;

* Null
* Boolean
* Number
* String
* Binary
* List
* Map
* Number Set
* String Set
* Binary Set

See [the documentation](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.NamingRulesDataTypes.html) to learn more data types of DynamoDB.

### Null

You can use `null` to express an attribute with an unknown or undefined state.

```bash
dy put 1 -i '{"null-field": null}'
```

### Boolean

A boolean type attribute can store either `true` or `false` to express a boolean value.

```bash
dy put 5 -i '{"true-field": true, "false-field": false}'
```

### Number

Numbers type express arbitrary numbers, including integers and fraction numbers up to 38 digits of precision.
The number must be decimal. You can use exponential notation.

```bash
dy put 10 -i '{"integer": 1, "fraction": 0.1, "minus": -3, "exponential": -1.23e-3}'
```

### String

Strings type represents an array of characters encoded with UTF-8.
You can use both single quotes and double quotes to express a string value.

```bash
dy put 15 -i '{"date":"2022-02-22T22:22:22Z"}'
dy put 16 -i "{'date':'2022-02-22T22:22:22Z'}"
```

You can use escape sequences if you use double quotes to express a string value.
The complete list of escape sequences is the following;

| Escape Sequence | Character Represented by Sequence |
|-----------------|-----------------------------------|
|       \0        | An ASCII NUL (X'00') character    |
|       \b        | A backspace character             |
|       \f        | A form feed character             |
|       \n        | A newline (linefeed) character    |
|       \r        | A carriage return character       |
|       \t        | A tab character                   |
|       \\\"      | A double quote (") character      |
|       \\\'      | A single quote (') character      |
|       \\\\      | A backslash (\\) character        |
|       \\/       | A slash (/) character             |
|     \\uXXXX     | An arbitrary unicode character    |

```bash
dy put 17 -i '{"escape":"\"hello\",\tworld!\n"}'
```

To escape an extended character that is not within the Basic Multilingual
Plane, the character is represented as a 12-character sequence,
encoded using the UTF-16 surrogate pair. For example, a string
containing only the G clef character (U+1D11E: 𝄞) may be represented as
`"\uD834\uDD1E"` as described in [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html).

On the other hand, you cannot use escape sequences if you use single quotes to express a string value.
String values are evaluated as is.
Because of shell behavior, you need special handling to input a single quote in the single-quoted argument.

```bash
dy put 18 -i '{"raw":'\''hello,\tworld!\n'\''}'
```

The above example creates an item with an attribute, `{"raw":{"S":"hello,\tworld!\n"}}`.
Or, you can use a heredoc.

```bash
dy put 19 -i "$(cat <<EOS
{
  "escape":"hello,\tworld!\n",
  "raw":'hello,\tworld!\n'
}
EOS
)"
```

### Binary
You can store any binary data as binary type. There are two types of literals.

When you use `b"<binary-data>"` style, you can use the following escape sequences.

| Escape Sequence | Character Represented by Sequence                    |
|-----------------|------------------------------------------------------|
| \0              | An ASCII NUL (X'00') character                       |
| \r              | A carriage return character                          |
| \n              | A newline (linefeed) character                       |
| \t              | A tab character                                      |
| \\\\            | A backslash (\\) character                           |
| \\\"            | A double quote (") character                         |
| \\\'            | A single quote (') character                         |
| \x41            | 7-bit character code (exactly 2 digits, up to 0x7F)  |

Additionally, you can skip leading spaces, including `\r`, `\n`, `\t` by putting a backslash at the end of a line.

input.json
```json
{
  "binary": b"Thi\x73 is a \
              bin.\r\n"
}
```

command
```bash
dy put 20 -i "$(cat input.json)"
```

When you use `b'<binary-data>'` style, binary data cannot span multiple lines.

### List
You can store an ordered collection of values using list type. Lists are enclosed in square brackets: `[ ... ]`.
A list is similar to a JSON array. There are no restrictions on the data types that can be stored in a list element, and the elements in a list element do not have to be of the same type.

The following example shows a list that contains two strings and a number.

```bash
dy put 25 -i '{"FavoriteThings": ["Cookies", "Coffee", 3.14159]}'
```

### Map
You can use Map type to store an unordered collection of name-value pairs.
Maps are enclosed in curly braces: `{ ... }`.
A map is similar to a JSON object.
There are no restrictions on the data types that can be stored in a map element,
and the elements in a map do not have to be the same type.

Maps are ideal for storing JSON documents in DynamoDB.
The following example shows a map that contains a string, a number, and a nested list that contains another map.

```bash
dy put 30 -i '{
    "Day": "Monday",
    "UnreadEmails": 42,
    "ItemsOnMyDesk": [
        "Coffee Cup",
        "Telephone",
        {
            "Pens": { "Quantity" : 3},
            "Pencils": { "Quantity" : 2},
            "Erasers": { "Quantity" : 1}
        }
    ]
}'
```

### Set
DynamoDB can represent sets of numbers, strings, or binary values.
Sets are represented by double angle brackets in dynein: `<< ... >>`.
All the elements within a set must be of the same type.
For example, a number set can only contain numbers, and a string set can only contain strings.

Dynein automatically infers the type of set based on its elements.

Each value within a set must be unique.
The order of the values within a set is not preserved.
Therefore, you must not rely on any particular order of elements within the set.
DynamoDB does not support empty sets; however, empty string and binary values are allowed within a set.

#### Number Set
In the following example, put an item containing a number set.

```bash
dy put 35 -i '{"number-set": <<0, -1, 1, 2>>}'
```

#### String Set
In the following example, put an item containing a string set.

```bash
dy put 36 -i '{"string-set": <<"0", "-1", "One", "Two">>}'
```

#### Binary Set
In the following example, put an item containing a binary set.

```bash
dy put 37 -i '{"binary-set": <<b"\x00", b"0x01", b"0x02">>}'
```
