# sql.md

## Conventions

- Write queries in lowercase.
- Place static queries to `<function_name>.sql` and construct dynamic queries directly in code.
- After adding or editing any query, run `task sqlx-prepare` so offline query data stays current.
- No PG enum type, use check constraints.
- Use a constraint to declare a primary key.
- Constraint template name is `<what>_<table1>_<col1>_.._<coln>`.
- Control `default` attribute through code.
