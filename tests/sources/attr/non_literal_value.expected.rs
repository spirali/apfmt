#[cfg_attr(
    feature = "__sqlite-shared",
    diesel(sql_type = crate::sql_types::TimestamptzSqlite),
)]
struct A;

#[cfg_attr(
    feature = "__sqlite",
    diesel(sql_type = crate::sql_types::TimestamptzSqlite),
    xxxx,
)]
struct B;
