# sql.md

## Conventions

- Write queries in lowercase.
- Use `<P>_<T>_<C>` pattern where `P` is constraint/index/action, `T` is n tables, `C` is n columns.
- Don't use inline constraints if there is available table constraint.
- After working with query, generate `sqlx` metadata to enable offline compile-time verification in sync.
- Use `CHECK` constraints instead of built-in enum types.
- Don't use column `default` attributes — control defaults from the code instead.
- Static queries go in `../queries/`; construct dynamic queries directly in code.
