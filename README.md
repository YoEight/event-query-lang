# EventQL Parser

A complete lexer and parser for EventQL (EQL), a query language designed for event sourcing systems.

## Features

- **Complete EventQL Support**: Parse queries with FROM, WHERE, GROUP BY, ORDER BY, LIMIT, and PROJECT clauses
- **Rich Expression Language**: Supports arithmetic, comparison, logical operators, and field access
- **Detailed Error Reporting**: Position-aware error messages with line and column numbers
- **Type-Safe AST**: Strongly-typed abstract syntax tree for query analysis and execution

## Quick Start

```rust
use eventql_parser::parse_query;

fn main() {
    let query = parse_query(
        "FROM e IN events WHERE e.id == 1 PROJECT INTO e"
    ).unwrap();

    println!("Parsed query: {:?}", query);
}
```

## EventQL Language

EventQL supports querying event streams with a SQL-like syntax:

```sql
FROM e IN events
WHERE e.price > 100 AND e.category == "electronics"
ORDER BY e.timestamp DESC
LIMIT 10
PROJECT INTO { id: e.id, price: e.price }
```

### Supported Clauses

- **FROM**: Define event sources (streams, subqueries)
- **WHERE**: Filter events with expressions
- **GROUP BY**: Group events by field or expression
- **ORDER BY**: Sort results (ASC/DESC)
- **LIMIT**: Limit number of results
- **PROJECT**: Define output shape

### Operators

- **Arithmetic**: `+`, `-`, `*`, `/`
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Logical**: `AND`, `OR`, `XOR`, `NOT`
- **Field Access**: `e.field`, `e.nested.field`

## Acknowledgements

This parser is based on the EventQL language specification from [EventSourcingDB] by The Native Web.
EventSourcingDB is an event store database that provides a powerful query language (EventQL) for querying and analyzing event streams in event-sourced systems.

[The EventQL Reference]

## License

MIT

[EventSourcingDB]: https://www.thenativeweb.io/products/eventsourcingdb
[The EventQL Reference]: https://docs.eventsourcingdb.io/reference/eventql/
